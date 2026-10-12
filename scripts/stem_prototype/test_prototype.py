"""Generated PCM tests. Synthetic sources/fakes are not neural inference."""

import argparse
import copy
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch
import numpy as np
import soundfile as sf
import cli
from common import (
    STEMS,
    audio,
    identity,
    mix_arrays,
    sha,
    validate_manifest,
    write_json,
)


class PrototypeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.directory = Path(self.temp.name)
        self.original = np.column_stack(
            (np.linspace(-0.1, 0.1, 128), np.zeros(128))
        ).astype("float32")
        self.sources = {
            name: self.original * (i + 1) / 21 for i, name in enumerate(sorted(STEMS))
        }
        sf.write(self.directory / "prepared.wav", self.original, 44100, subtype="FLOAT")
        prep = {
            "version": 1,
            "prepared_rate": 44100,
            "channels": 2,
            "prepared_frames": 128,
            "prepared_sha256": sha(self.directory / "prepared.wav"),
            "source_rate": 44100,
            "source_frames": 128,
            "source_span": {"start": 0, "end_exclusive": 128},
        }
        self.manifest = {
            "version": 1,
            "status": "complete",
            "identity": {"preparation": prep},
            "stems": {},
        }
        self.manifest["job_key"] = identity(self.manifest["identity"])
        for name, value in self.sources.items():
            sf.write(self.directory / (name + ".wav"), value, 44100, subtype="FLOAT")
            self.manifest["stems"][name] = {
                "file": name + ".wav",
                "sha256": sha(self.directory / (name + ".wav")),
                "rate": 44100,
                "channels": 2,
                "frames": 128,
            }
        self.path = self.directory / "manifest.json"
        write_json(self.path, self.manifest)

    def tearDown(self):
        self.temp.cleanup()

    def test_validated_cache_invalidates_changed_identity_or_pcm(self):
        validate_manifest(self.path, self.manifest["identity"])
        expected = copy.deepcopy(self.manifest["identity"])
        expected["model"] = "different"
        with self.assertRaises(ValueError):
            validate_manifest(self.path, expected)
        with (self.directory / "guitar.wav").open("ab") as stream:
            stream.write(b"corruption")
        with self.assertRaises(ValueError):
            validate_manifest(self.path)

    def test_rejects_future_manifest_traversal_nonfinite_and_wrong_length(self):
        for mutation in (
            lambda m: m.update(version=2),
            lambda m: m["stems"]["guitar"].update(file="../outside.wav"),
            lambda m: m.update(status="cancelled"),
        ):
            candidate = copy.deepcopy(self.manifest)
            mutation(candidate)
            write_json(self.path, candidate)
            with self.assertRaises(ValueError):
                validate_manifest(self.path)
        for sample in (np.full((128, 2), np.nan), np.zeros((127, 2))):
            sf.write(self.directory / "invalid.wav", sample, 44100, subtype="FLOAT")
            with self.assertRaises(ValueError):
                audio(self.directory / "invalid.wav", 128)

    def test_declared_geometry_and_span_are_validated(self):
        candidate = copy.deepcopy(self.manifest)
        candidate["stems"]["guitar"]["frames"] = 127
        write_json(self.path, candidate)
        with self.assertRaises(ValueError):
            validate_manifest(self.path)
        candidate = copy.deepcopy(self.manifest)
        candidate["identity"]["preparation"]["source_span"]["end_exclusive"] = 129
        candidate["job_key"] = identity(candidate["identity"])
        write_json(self.path, candidate)
        with self.assertRaises(ValueError):
            validate_manifest(self.path)

    def test_mix_preserves_existing_sidecar(self):
        output = self.directory / "custom.wav"
        sidecar = output.with_suffix(".mix.json")
        sidecar.write_text("existing user data")
        args = argparse.Namespace(
            manifest=self.path, preset="sum", gain=[], output=output
        )
        with self.assertRaises(ValueError):
            cli.mix(args)
        self.assertFalse(output.exists())
        self.assertEqual(sidecar.read_text(), "existing user data")

    def test_short_roformer_padding_preserves_timeline_and_declares_crop(self):
        from common import roformer_input

        prepared, corrections = roformer_input(self.original, 256)
        np.testing.assert_array_equal(prepared[:, :128].T, self.original)
        np.testing.assert_array_equal(prepared[:, 128:], np.zeros((2, 128)))
        self.assertEqual(corrections[0]["right_padding_frames"], 128)
        self.assertEqual(corrections[0]["crop_end_exclusive"], 128)
        unchanged, corrections = roformer_input(self.original, 64)
        np.testing.assert_array_equal(unchanged.T, self.original)
        self.assertEqual(corrections, [])

    def test_mix_math_residual_and_gain_policy(self):
        result, gain, _ = mix_arrays(self.sources, self.original, "sum", {})
        np.testing.assert_allclose(result, self.original, atol=1e-8)
        self.assertEqual(gain, 1)
        result, _, _ = mix_arrays(self.sources, self.original, "no-guitar", {})
        np.testing.assert_allclose(
            result, self.original - self.sources["guitar"], atol=1e-8
        )
        result, _, _ = mix_arrays(
            self.sources, self.original, "original-minus-guitar", {}
        )
        np.testing.assert_allclose(
            result, self.original - self.sources["guitar"], atol=1e-8
        )
        louder = {name: value * 100 for name, value in self.sources.items()}
        result, gain, peak = mix_arrays(louder, self.original, "sum", {})
        self.assertAlmostEqual(float(np.abs(result).max()), 0.98, places=6)
        self.assertAlmostEqual(gain, 0.98 / peak)
        with self.assertRaises(ValueError):
            mix_arrays(self.sources, self.original, "sum", {"guitar": float("nan")})

    def test_failed_worker_never_publishes_and_discards_staging(self):
        args = argparse.Namespace(
            start=0,
            duration=1,
            whole=False,
            model="demucs-6s",
            models_dir=self.directory,
            jobs_dir=self.directory / "jobs",
            threads=1,
            device="cpu",
            input=self.directory / "input.wav",
        )

        def fake_child(command, env=None):
            if command[0] == "cargo":
                out = Path(command[-3])
                out.mkdir()
                shutil = __import__("shutil")
                shutil.copyfile(self.directory / "prepared.wav", out / "prepared.wav")
                write_json(
                    out / "preparation.json", self.manifest["identity"]["preparation"]
                )
            else:
                raise KeyboardInterrupt

        with (
            patch.object(cli, "check_models", return_value={}),
            patch.object(cli, "runtime_identity", return_value={}),
            patch.object(cli, "run_child", side_effect=fake_child),
        ):
            with self.assertRaises(KeyboardInterrupt):
                cli.separate(args)
        self.assertEqual(list(args.jobs_dir.iterdir()), [])

    @unittest.skipIf(
        os.name == "nt", "POSIX signal lifecycle test; Windows remains unverified"
    )
    def test_sigterm_reaps_child(self):
        pidfile = self.directory / "pid"
        runner = self.directory / "runner.py"
        runner.write_text(
            "import sys;sys.path.insert(0,"
            + repr(str(Path(__file__).parent))
            + ");from cli import run_child;run_child([sys.executable,'-c',"
            + repr(
                "import os,time;open("
                + repr(str(pidfile))
                + ",'w').write(str(os.getpid()));time.sleep(30)"
            )
            + "])\n"
        )
        parent = subprocess.Popen(
            [sys.executable, str(runner)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        try:
            deadline = time.monotonic() + 5
            while not pidfile.exists() and time.monotonic() < deadline:
                time.sleep(0.02)
            self.assertTrue(pidfile.exists())
            child_pid = int(pidfile.read_text())
            parent.send_signal(signal.SIGTERM)
            parent.wait(timeout=5)
            with self.assertRaises(ProcessLookupError):
                os.kill(child_pid, 0)
        finally:
            if parent.poll() is None:
                parent.kill()
                parent.wait()


if __name__ == "__main__":
    unittest.main()
