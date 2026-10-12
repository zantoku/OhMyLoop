import {bridge} from './bridge';
import type {Snapshot} from './types';
type Target={referenceToken:string;songKey?:string};
// Capture the reference before a file picker opens, including unloaded library songs.
// The native boundary revalidates this token before it publishes a replacement.
export class RecordingReplacement {
  private target:Target|null=null;
  active(referenceToken:string){this.target={referenceToken};}
  async library(songKey:string){
    this.target=null;
    const referenceToken=await bridge.libraryReference(songKey);
    this.target={songKey,referenceToken};
  }
  async replace(timingUnchanged:boolean):Promise<Snapshot|null>{
    const target=this.target;
    if(!target)throw new Error('Select a recording to replace first.');
    return target.songKey
      ?bridge.replaceLibraryReference(target.songKey,target.referenceToken,timingUnchanged)
      :bridge.replaceRecording(target.referenceToken,timingUnchanged);
  }
}
