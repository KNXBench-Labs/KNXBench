/** Preserves untouched literal text while adapting textarea line endings. */
export function textareaText(value:string):string{return value.replace(/\r\n|\r/g,"\n");}
function high(code:number):boolean{return code>=0xd800&&code<=0xdbff;}
function low(code:number):boolean{return code>=0xdc00&&code<=0xdfff;}
function rawOffset(value:string,logicalOffset:number):number{
 let raw=0;let logical=0;
 while(raw<value.length&&logical<logicalOffset){
  raw+=value[raw]==="\r"&&value[raw+1]==="\n"?2:1;
  logical++;
 }
 return raw;
}
export function applyTextareaEdit(before:string,next:string):string{
 const old=textareaText(before);const current=textareaText(next);
 if(old===current)return before;
 let start=0;
 while(start<old.length&&start<current.length&&old[start]===current[start])start++;
 // A shared high surrogate is not a complete shared Unicode character.
 if(start>0&&((high(old.charCodeAt(start-1))&&low(old.charCodeAt(start)))||
  (high(current.charCodeAt(start-1))&&low(current.charCodeAt(start)))))start--;
 let oldEnd=old.length;let newEnd=current.length;
 while(oldEnd>start&&newEnd>start&&old[oldEnd-1]===current[newEnd-1]){oldEnd--;newEnd--;}
 if(oldEnd<old.length&&oldEnd>0&&high(old.charCodeAt(oldEnd-1))&&low(old.charCodeAt(oldEnd))){oldEnd++;newEnd++;}
 const ending=before.match(/\r\n|\r|\n/)?.[0]??"\n";
 const replacement=current.slice(start,newEnd).replace(/\n/g,ending);
 return before.slice(0,rawOffset(before,start))+replacement+before.slice(rawOffset(before,oldEnd));
}
