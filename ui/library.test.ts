import {describe,expect,it} from 'vitest';
import {librarySongs,librarySorts,songLength,sortsDescending,type LibraryEntry,type LibrarySort} from './library';
const a:LibraryEntry={song_key:'a',metadata:{title:'Riff 10',artist:'Zed',album:'Somethin’ Else'},file_location:'/music/b.wav',date_added:1,last_opened:9,duration_ms:300_000};
const b:LibraryEntry={song_key:'b',metadata:{title:'Riff 2',artist:'Alpha',album:'Kind of Blue'},file_location:'/music/a.wav',date_added:3,last_opened:7,duration_ms:90_000};
describe('library presentation',()=>{
  it.each<[LibrarySort,string[]]>([['title',['b','a']],['artist',['b','a']],['duration_ms',['b','a']],['date_added',['a','b']],['last_opened',['b','a']]])('sorts %s in both directions without mutating source data',(field,keys)=>{
    const source=[a,b];expect(librarySongs(source,'',field,false).map(e=>e.song_key)).toEqual(keys);expect(librarySongs(source,'',field,true).map(e=>e.song_key)).toEqual([...keys].reverse());expect(source).toEqual([a,b]);
  });
  it('offers title, artist, length, date added and last opened, dates newest first',()=>{
    expect(librarySorts.map(sort=>sort.label)).toEqual(['Last opened','Title','Artist','Length','Date added']);
    expect(librarySorts.filter(sort=>sortsDescending(sort.field)).map(sort=>sort.field)).toEqual(['last_opened','date_added']);
  });
  it('searches across metadata and paths and keeps unknown values last',()=>{
    expect(librarySongs([a,b],'ALPHA a.wav','title',false)).toEqual([b]);
    expect(librarySongs([a,b],'somethin','title',false)).toEqual([a]);
    const unknown={...b,song_key:'c',metadata:{...b.metadata,artist:''},duration_ms:0};
    for(const reverse of [false,true])expect(librarySongs([unknown,a],'','artist',reverse)).toEqual([a,unknown]);
    for(const reverse of [false,true])expect(librarySongs([unknown,a],'','duration_ms',reverse)).toEqual([a,unknown]);
  });
  it('shows lengths as m:ss or h:mm:ss, and a dash when unknown',()=>{
    expect(songLength(658_400)).toBe('10:58');expect(songLength(59_600)).toBe('1:00');expect(songLength(3_725_000)).toBe('1:02:05');expect(songLength(0)).toBe('—');
  });
});
