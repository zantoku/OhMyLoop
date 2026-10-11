import {describe,expect,it} from 'vitest';
import {librarySongs,type LibraryEntry,type LibrarySort} from './library';
const a:LibraryEntry={song_key:'a',metadata:{title:'Riff 10',artist:'Zed',album:'Album A'},file_location:'/music/b.wav',date_added:1,last_opened:9};
const b:LibraryEntry={song_key:'b',metadata:{title:'Riff 2',artist:'Alpha',album:'Album B'},file_location:'/music/a.wav',date_added:3,last_opened:7};
describe('library presentation',()=>{
  it.each<[LibrarySort,string[]]>([['title',['b','a']],['artist',['b','a']],['album',['a','b']],['file_location',['b','a']],['date_added',['a','b']],['last_opened',['b','a']]])('sorts %s in both directions without mutating source data',(field,keys)=>{
    const source=[a,b];expect(librarySongs(source,'',field,false).map(e=>e.song_key)).toEqual(keys);expect(librarySongs(source,'',field,true).map(e=>e.song_key)).toEqual([...keys].reverse());expect(source).toEqual([a,b]);
  });
  it('searches across metadata and paths and keeps unknown values last',()=>{
    expect(librarySongs([a,b],'ALPHA a.wav','title',false)).toEqual([b]);
    const unknown={...b,song_key:'c',metadata:{...b.metadata,artist:''}};
    for(const reverse of [false,true])expect(librarySongs([unknown,a],'','artist',reverse)).toEqual([a,unknown]);
  });
});
