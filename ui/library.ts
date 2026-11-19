// Library DTOs and presentation-only ordering. Rust owns persistence and identity.
export type SongMetadata = {artist:string;title:string;album:string};
/** A song as the catalogue stores it. */
export type CatalogueEntry = {song_key:string;metadata:SongMetadata;file_location:string;date_added:number;last_opened:number};
/** A row of the song list: the stored entry plus the recording's length, read when the list loads (0 when unreadable). */
export type LibraryEntry = CatalogueEntry & {duration_ms:number};
export type Catalogue = {version:number;songs:LibraryEntry[]};
export type LibrarySort = 'title' | 'artist' | 'duration_ms' | 'date_added' | 'last_opened';
/** The sort choices, in menu order. Search still matches album and file location. */
export const librarySorts:readonly {field:LibrarySort;label:string}[]=[
  {field:'last_opened',label:'Last opened'},{field:'title',label:'Title'},{field:'artist',label:'Artist'},
  {field:'duration_ms',label:'Length'},{field:'date_added',label:'Date added'}
];
/** Dates list the newest first; names and lengths start from A and the shortest. */
export const sortsDescending=(field:LibrarySort)=>field==='date_added'||field==='last_opened';
const collator=new Intl.Collator(undefined,{numeric:true,sensitivity:'base'});
export function librarySongs(entries:readonly LibraryEntry[],query:string,field:LibrarySort,descending:boolean):LibraryEntry[]{
  const words=query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  const value=(entry:LibraryEntry)=>field==='title'||field==='artist'?entry.metadata[field]:entry[field];
  return entries.filter(entry=>{
    const text=[entry.metadata.title,entry.metadata.artist,entry.metadata.album,entry.file_location].join(' ').toLocaleLowerCase();
    return words.every(word=>text.includes(word));
  }).sort((a,b)=>{
    const av=value(a),bv=value(b);
    // Unknown artists and lengths consistently follow known ones in either direction.
    const unknown=(v:string|number)=>v===''||v===0&&field==='duration_ms';
    if(unknown(av)&&!unknown(bv))return 1;if(unknown(bv)&&!unknown(av))return -1;
    const order=typeof av==='number'&&typeof bv==='number'?av-bv:collator.compare(String(av),String(bv));
    return (descending?-order:order)||collator.compare(a.metadata.title,b.metadata.title)||a.song_key.localeCompare(b.song_key);
  });
}
/** The last part of a file path, for either path separator. */
export const fileName=(path:string)=>path.split(/[\\/]/).filter(Boolean).pop()??path;
export function libraryDate(milliseconds:number):string {
  return new Intl.DateTimeFormat(undefined,{year:'numeric',month:'short',day:'numeric',hour:'2-digit',minute:'2-digit'}).format(milliseconds);
}
/** A song's length as m:ss or h:mm:ss; a dash when unknown. */
export function songLength(milliseconds:number):string {
  if(!(milliseconds>0))return '—';
  const total=Math.round(milliseconds/1000),hours=Math.floor(total/3600),minutes=Math.floor(total/60)%60,seconds=String(total%60).padStart(2,'0');
  return hours?`${hours}:${String(minutes).padStart(2,'0')}:${seconds}`:`${minutes}:${seconds}`;
}
