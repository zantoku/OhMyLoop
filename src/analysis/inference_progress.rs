//! Observe the public model interface without changing upstream inference.
use super::progress::{ProgressUnit, ProgressUpdate};
use anyhow::Result;
use beat_this::{Model, Tensor};
use std::collections::HashMap;

pub struct ObservedModel<M> {
    model: M,
    observer: Option<SegmentObserver>,
}
struct SegmentObserver {
    completed: u64,
    total: u64,
    publish: Box<dyn FnMut(ProgressUpdate) -> Result<()>>,
}
// beat-this =1.1.0 produces 1 + samples/441 mel frames and predicts with
// stride 1500 - 2*6. Tests exercise this count through its public pipeline.
pub fn segment_count(samples: u64) -> u64 {
    (1 + samples / 441).div_ceil(1488)
}
impl<M> ObservedModel<M> {
    pub fn plain(model: M) -> Self {
        Self {
            model,
            observer: None,
        }
    }
    pub fn beats(
        model: M,
        samples: u64,
        publish: impl FnMut(ProgressUpdate) -> Result<()> + 'static,
    ) -> Self {
        Self {
            model,
            observer: Some(SegmentObserver {
                completed: 0,
                total: segment_count(samples),
                publish: Box::new(publish),
            }),
        }
    }
}
impl<M: Model> Model for ObservedModel<M> {
    fn run(&mut self, inputs: &[(&str, &Tensor)]) -> Result<HashMap<String, Tensor>> {
        if let Some(observer) = &mut self.observer {
            (observer.publish)(ProgressUpdate::measured(
                "Finding beats and downbeats",
                observer.completed,
                Some(observer.total),
                ProgressUnit::Segments,
            ))?;
        }
        let outputs = self.model.run(inputs)?;
        if let Some(observer) = &mut self.observer {
            observer.completed += 1;
            (observer.publish)(ProgressUpdate::measured(
                "Finding beats and downbeats",
                observer.completed,
                Some(observer.total),
                ProgressUnit::Segments,
            ))?;
        }
        Ok(outputs)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use beat_this::BeatThis;
    use std::{cell::RefCell, rc::Rc};
    struct FakeModel;
    impl Model for FakeModel {
        fn run(&mut self, inputs: &[(&str, &Tensor)]) -> Result<HashMap<String, Tensor>> {
            let (name, input) = inputs[0];
            Ok(if name == "audio_pcm" {
                let time = 1 + input.data.len() / 441;
                HashMap::from([(
                    "mel_spectrogram".into(),
                    Tensor {
                        shape: vec![1, time, 128],
                        data: vec![0.0; time * 128],
                    },
                )])
            } else {
                let time = input.shape[1];
                ["beat", "downbeat"]
                    .map(|name| {
                        (
                            name.into(),
                            Tensor {
                                shape: vec![1, time],
                                data: vec![-10.0; time],
                            },
                        )
                    })
                    .into()
            })
        }
    }
    #[test]
    fn reports_actual_completed_segments_without_changing_predictions() {
        for frames in [100, 1488, 1500, 2000, 5000] {
            let samples = (frames - 1) * 441;
            let updates = Rc::new(RefCell::new(vec![]));
            let capture = updates.clone();
            let mut tracker = BeatThis::from_models(
                ObservedModel::plain(FakeModel),
                ObservedModel::beats(FakeModel, samples as u64, move |u| {
                    capture.borrow_mut().push(u);
                    Ok(())
                }),
            );
            let result = tracker.analyze_audio(&vec![0.0; samples], 22_050).unwrap();
            assert_eq!(result.beat_logits, vec![-10.0; frames]);
            let updates = updates.borrow();
            let last = updates.last().unwrap().progress.as_ref().unwrap();
            assert_eq!(last.completed, segment_count(samples as u64));
            assert_eq!(last.total, Some(last.completed));
            assert_eq!(
                updates
                    .first()
                    .unwrap()
                    .progress
                    .as_ref()
                    .unwrap()
                    .completed,
                0
            );
        }
    }
}
