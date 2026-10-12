/** Isolated preview-first notebook; authenticated services own all mutations. */
import { useEffect, useId, useRef, useState } from "react";
import { createNotesClient, sameNotesContext, sameNoteDraft, validNoteDraft, noteTargetKey, type NotesApplied, type NotesClient, type NotesContext, type NoteTarget, type NotesView, type NoteDraft, type NoteAction, type NotePreview } from "./client";
import { applyTextareaEdit, textareaText } from "./text";
import { noteCopy } from "./copy";
import HelpTip from "../../HelpTip";
import { requestHelpTopic } from "../../help";
import styles from "./editor.module.css";
export type NoteTargetChoice=Readonly<{target:NoteTarget;label:string}>;
export type ProjectNotesEditorProps=Readonly<{context:NotesContext;language?:"en"|"de";targetChoices?:readonly NoteTargetChoice[];client?:NotesClient;onApplied?:(outcome:NotesApplied)=>void|Promise<void>;onDirtyChange?:(dirty:boolean)=>void}>;
const defaultClient=createNotesClient();
const emptyDraft=():NoteDraft=>({target:{kind:"project"},title:"",body:"",includeInDocumentation:false});
type SwitchIntent={kind:"select";id:number}|{kind:"new"}|{kind:"reload"};
export function ProjectNotesEditor({context,client=defaultClient,language="en",targetChoices=[],onApplied,onDirtyChange}:ProjectNotesEditorProps){
 const id=useId();const copy=noteCopy[language];
 const [view,setView]=useState<NotesView|null>(null);const [draft,setDraft]=useState<NoteDraft|null>(null);
 const [selected,setSelected]=useState<number|null>(null);const [dirty,setDirty]=useState(false);const dirtyRef=useRef(false);
 const [pending,setPending]=useState<NotePreview|null>(null);const pendingRef=useRef<NotePreview|null>(null);
 const [busy,setBusy]=useState(false);const [error,setError]=useState<string|null>(null);const [notice,setNotice]=useState<string|null>(null);
 const [stale,setStale]=useState(false);const [intent,setIntent]=useState<SwitchIntent|null>(null);const [filter,setFilter]=useState("");
 const sequence=useRef(0);const applying=useRef(false);const request=useRef<AbortController|null>(null);const currentOwner=useRef({context,client});
 const dirtyCallback=useRef(onDirtyChange);dirtyCallback.current=onDirtyChange;
 const current=view!==null&&sameNotesContext(view,context)&&!stale;
 const writable=current&&!!onApplied&&!busy;
 const stored=view?.notes.find(note=>note.id===selected);
 const choices=[{target:{kind:"project"} as NoteTarget,label:copy.project},...targetChoices.filter(choice=>choice.target.kind!=="project")];
 const targetLabel=(target:NoteTarget)=>choices.find(choice=>noteTargetKey(choice.target)===noteTargetKey(target))?.label??`${copy.lostTarget}: ${noteTargetKey(target)}`;
 function markDirty(value:boolean){dirtyRef.current=value;setDirty(value);dirtyCallback.current?.(value);}
 function revoke(){
  const old=pendingRef.current;pendingRef.current=null;setPending(null);
  if(old)void client.cancel(old.token).catch(()=>undefined);
 }
 function choose(value:NotesView,noteId:number|null){
  const note=value.notes.find(entry=>entry.id===noteId);setSelected(note?.id??null);
  setDraft(note?{target:note.target,title:note.title,body:note.body,includeInDocumentation:note.includeInDocumentation}:emptyDraft());markDirty(false);
 }
 async function load(){
  const generation=++sequence.current;request.current?.abort();const controller=new AbortController();request.current=controller;
  const owner={context,client};currentOwner.current=owner;revoke();setBusy(true);setError(null);setNotice(null);
  try{
   const value=await client.read(controller.signal);
   if(controller.signal.aborted||generation!==sequence.current)return;
   if(!sameNotesContext(value,owner.context))throw new Error(copy.changed);
   setView(value);choose(value,value.notes[0]?.id??null);setStale(false);
  }catch(reason){if(!controller.signal.aborted&&generation===sequence.current){setError(reason instanceof Error?reason.message:copy.changed);setStale(true);}}
  finally{if(generation===sequence.current)setBusy(false);}
 }
 useEffect(()=>{
  currentOwner.current={context,client};
  if(dirtyRef.current){sequence.current++;request.current?.abort();revoke();setBusy(false);setStale(true);setError(copy.changed);}
  else void load();
  return()=>{sequence.current++;request.current?.abort();const old=pendingRef.current;pendingRef.current=null;if(old)void client.cancel(old.token).catch(()=>undefined);};
  // Identity, not freshly allocated host objects, owns this editing lifetime.
  // eslint-disable-next-line react-hooks/exhaustive-deps
 },[client,context.serverIncarnation,context.projectIncarnation,context.snapshotRevision]);
 // A refusal must be visible where the user acted, not above a narrow viewport.
 const alertRef=useRef<HTMLParagraphElement|null>(null);
 useEffect(()=>{if(error)alertRef.current?.scrollIntoView?.({block:"nearest"});},[error]);
 useEffect(()=>{
  if(!pending)return;
  const timer=setTimeout(()=>{if(pendingRef.current?.token===pending.token){revoke();setNotice(copy.expired);}},pending.expiresInSeconds*1000);
  return()=>clearTimeout(timer);
  // eslint-disable-next-line react-hooks/exhaustive-deps
 },[pending,copy.expired]);
 function edit(next:NoteDraft){if(!writable)return;sequence.current++;request.current?.abort();revoke();setDraft(next);setError(null);setNotice(null);markDirty(stored?!sameNoteDraft(stored,next):true);}
 function performSwitch(next:SwitchIntent){
  revoke();setIntent(null);setError(null);setNotice(null);
  if(next.kind==="reload"){markDirty(false);void load();return;}
  if(!view)return;
  if(next.kind==="new"){setSelected(null);setDraft(emptyDraft());markDirty(true);}
  else choose(view,next.id);
 }
 function switchTo(next:SwitchIntent){if(busy)return;if(dirty) setIntent(next);else performSwitch(next);}
 async function preview(action:NoteAction){
  if(!writable||!view)return;
  const generation=++sequence.current;request.current?.abort();const controller=new AbortController();request.current=controller;revoke();setBusy(true);setError(null);setNotice(null);
  const owner={context,client};
  try{
   const value=await client.preview(context,action,controller.signal);
   if(controller.signal.aborted||generation!==sequence.current||!sameNotesContext(currentOwner.current.context,owner.context)||currentOwner.current.client!==client){void client.cancel(value.token).catch(()=>undefined);return;}
   pendingRef.current=value;setPending(value);
  }catch(reason){if(!controller.signal.aborted&&generation===sequence.current)setError(reason instanceof Error?reason.message:copy.changed);}
  finally{if(generation===sequence.current)setBusy(false);}
 }
 async function confirm(){
  const plan=pendingRef.current;if(!plan||!writable||!onApplied||applying.current)return;
  applying.current=true;
  const generation=++sequence.current;const controller=new AbortController();request.current=controller;setBusy(true);setError(null);
  let committed=false;
  try{
   const outcome=await client.apply(plan.token,controller.signal);
   if(controller.signal.aborted||generation!==sequence.current)return;
   committed=true;pendingRef.current=null;setPending(null);markDirty(false);setStale(true);setNotice(copy.committed);
   await onApplied(outcome);
  }catch(reason){
   if(controller.signal.aborted||generation!==sequence.current)return;
   if(committed){setError(copy.refreshFailed);setStale(true);}
   else if(typeof reason==="object"&&reason!==null&&"kind" in reason&&typeof reason.kind==="string"){revoke();setError(reason instanceof Error?reason.message:copy.changed);}
   else {revoke();setStale(true);setError(copy.unknown);}
  }finally{applying.current=false;if(generation===sequence.current)setBusy(false);}
 }
 const reviewAfter=pending&&(pending.action.kind==="create"||pending.action.kind==="replace")?pending.action.draft:null;
 return <section className={styles.editor} aria-labelledby={`${id}-heading`} aria-busy={busy}>
  <header className={styles.header}><h3 id={`${id}-heading`}>{copy.heading}</h3><button type="button" onClick={()=>requestHelpTopic("workbench")}>{copy.help}</button><button type="button" disabled={busy} onClick={()=>switchTo({kind:"reload"})}>{copy.reload}</button></header>
  <p className={styles.guidance}>{copy.guidance}</p>
  {view&&<p className={styles.guidance}>{view.persistence==="session-only"?copy.session:copy.native}</p>}
  {!onApplied&&<p className={styles.notice}>{copy.readonly}</p>}
  {!view&&!error&&<p role="status">{copy.loading}</p>}{error&&<p ref={alertRef} className={styles.notice} role="alert">{error}</p>}{notice&&<p className={styles.notice} role="status">{notice}</p>}
  {intent&&<div className={styles.notice} role="alert"><p>{copy.discardPrompt}</p><div className={styles.toolbar}><button type="button" onClick={()=>performSwitch(intent)}>{copy.discard}</button><button type="button" onClick={()=>setIntent(null)}>{copy.keep}</button></div></div>}
  {view&&<div className={styles.layout}>
   <nav className={styles.list} aria-label={copy.heading}>
    <label htmlFor={`${id}-filter`}>{copy.filter}</label><input id={`${id}-filter`} type="search" value={filter} onChange={event=>setFilter(event.target.value)}/>
    <button type="button" disabled={!writable} onClick={()=>switchTo({kind:"new"})}>{copy.newNote}</button>
    <ul>{view.notes.filter(note=>`${note.title}\n${note.body}`.toLocaleLowerCase(language).includes(filter.toLocaleLowerCase(language))).map(note=><li key={note.id}><button type="button" data-note-id={note.id} aria-current={selected===note.id} disabled={busy||!current} onClick={()=>switchTo({kind:"select",id:note.id})}>{note.title}</button></li>)}</ul>
    {!view.notes.length&&<p>{copy.empty}</p>}
   </nav>
   {draft&&<div className={styles.form}>
    <label htmlFor={`${id}-title`}>{copy.title}</label><textarea id={`${id}-title`} value={textareaText(draft.title)} readOnly={!writable} onChange={event=>edit({...draft,title:applyTextareaEdit(draft.title,event.target.value)})}/>
    <label htmlFor={`${id}-body`}>{copy.body}</label><textarea className={styles.body} id={`${id}-body`} value={textareaText(draft.body)} readOnly={!writable} onChange={event=>edit({...draft,body:applyTextareaEdit(draft.body,event.target.value)})}/>
    <label htmlFor={`${id}-target`}>{copy.target}</label>{draft.target.kind==="groupRange"&&<HelpTip labelKey="help.tip.groupRange.label" textKey="help.tip.groupRange.text" topicId="groupRanges"/>}<select id={`${id}-target`} value={noteTargetKey(draft.target)} disabled={!writable} onChange={event=>{const choice=choices.find(entry=>noteTargetKey(entry.target)===event.target.value);if(choice)edit({...draft,target:choice.target});}}>
     {!choices.some(choice=>noteTargetKey(choice.target)===noteTargetKey(draft.target))&&<option value={noteTargetKey(draft.target)}>{targetLabel(draft.target)}</option>}
     {choices.map(choice=><option key={noteTargetKey(choice.target)} value={noteTargetKey(choice.target)}>{choice.label}</option>)}
    </select>
    <label className={styles.check}><input type="checkbox" checked={draft.includeInDocumentation} disabled={!writable} onChange={event=>edit({...draft,includeInDocumentation:event.target.checked})}/>{copy.include}</label>
    <div className={styles.toolbar}>
     <button type="button" disabled={!writable||!dirty||!validNoteDraft(draft)} onClick={()=>void preview(selected===null?{kind:"create",position:view.notes.length,draft}:{kind:"replace",id:selected,draft})}>{copy.preview}</button>
     {stored&&<><button type="button" disabled={!writable||dirty} onClick={()=>void preview({kind:"remove",id:stored.id})}>{copy.remove}</button>
      <button type="button" disabled={!writable||dirty||stored.position===0} onClick={()=>void preview({kind:"move",id:stored.id,position:stored.position-1})}>{copy.up}</button>
      <button type="button" disabled={!writable||dirty||stored.position>=view.notes.length-1} onClick={()=>void preview({kind:"move",id:stored.id,position:stored.position+1})}>{copy.down}</button></>}
    </div>
    {dirty&&!validNoteDraft(draft)&&<p className={styles.guidance}>{copy.invalid}</p>}
   </div>}
  </div>}
  {pending&&<section className={styles.review} aria-label={copy.review}><h4>{copy.review}</h4><p>{copy.kind}: {pending.action.kind} · {pending.createdId??pending.previous?.id}</p>
   {pending.noOp&&<p>{copy.noop}</p>}
   <div className={styles.preview}><div><h5>{copy.before}</h5>{pending.previous&&<><p className={styles.literal}>{pending.previous.title}</p><p className={styles.literal}>{pending.previous.body}</p><p>{targetLabel(pending.previous.target)} · {copy.include}: {String(pending.previous.includeInDocumentation)}</p><p>#{pending.previous.position}</p></>}</div>
    <div><h5>{copy.after}</h5>{reviewAfter&&<><p className={styles.literal}>{reviewAfter.title}</p><p className={styles.literal}>{reviewAfter.body}</p><p>{targetLabel(reviewAfter.target)} · {copy.include}: {String(reviewAfter.includeInDocumentation)}</p></>}{pending.action.kind==="move"&&<p>#{pending.action.position}</p>}</div></div>
   <div className={styles.toolbar}><button type="button" disabled={!writable} onClick={()=>void confirm()}>{copy.confirm}</button><button type="button" disabled={busy} onClick={revoke}>{copy.cancel}</button></div>
  </section>}
 </section>;
}
