// Library DTOs and presentation-only ordering. Rust owns persistence and identity.
export type SongMetadata = {artist:string;title:string;album:string};
export type LibraryEntry = {song_key:string;metadata:SongMetadata;file_location:string;date_added:number;last_opened:number};
export type Catalogue = {version:number;songs:LibraryEntry[]};
export type LibrarySort = keyof SongMetadata | 'file_location' | 'date_added' | 'last_opened';
export const libraryColumns:readonly {field:LibrarySort;label:string}[]=[
  {field:'title',label:'Title'},{field:'artist',label:'Artist'},{field:'album',label:'Album'},
  {field:'file_location',label:'File location'},{field:'date_added',label:'Date added'},{field:'last_opened',label:'Last opened'}
];
const collator=new Intl.Collator(undefined,{numeric:true,sensitivity:'base'});
export function librarySongs(entries:readonly LibraryEntry[],query:string,field:LibrarySort,descending:boolean):LibraryEntry[]{
  const words=query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  const value=(entry:LibraryEntry)=>field in entry.metadata?entry.metadata[field as keyof SongMetadata]:entry[field as 'file_location'|'date_added'|'last_opened'];
  return entries.filter(entry=>{
    const text=[entry.metadata.title,entry.metadata.artist,entry.metadata.album,entry.file_location].join(' ').toLocaleLowerCase();
    return words.every(word=>text.includes(word));
  }).sort((a,b)=>{
    const av=value(a),bv=value(b);
    // Unknown artist/album consistently follow populated fields in either direction.
    if(av===''&&bv!=='')return 1;if(bv===''&&av!=='')return -1;
    const order=typeof av==='number'&&typeof bv==='number'?av-bv:collator.compare(String(av),String(bv));
    return (descending?-order:order)||collator.compare(a.metadata.title,b.metadata.title)||a.song_key.localeCompare(b.song_key);
  });
}
export function libraryDate(milliseconds:number):string {
  return new Intl.DateTimeFormat(undefined,{year:'numeric',month:'short',day:'numeric',hour:'2-digit',minute:'2-digit'}).format(milliseconds);
}
