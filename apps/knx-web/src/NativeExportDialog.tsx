/** Native export confirmation discloses retained foreign data before downloading. */
import {useRef} from "react";
import type {ProjectTree} from "./bindings/ProjectTree";
import Overlay from "./Overlay";
import {useTranslate} from "./i18n";
import {nativeExportMatches,nativeExportPath} from "./nativeExport";
import "./NativeExportDialog.css";

export default function NativeExportDialog(props:{reviewed:ProjectTree,current:ProjectTree|null,onClose:()=>void}) {
  const t=useTranslate();const cancel=useRef<HTMLButtonElement|null>(null);const started=useRef(false);
  const available=nativeExportMatches(props.reviewed,props.current);
  function download() {
    if(started.current||!nativeExportMatches(props.reviewed,props.current))return;
    started.current=true;
    const anchor=document.createElement("a");anchor.href=nativeExportPath(props.reviewed);anchor.download="project.knxdb";
    document.body.append(anchor);
    try {anchor.click();} finally {anchor.remove();}
    props.onClose();
  }
  return <Overlay labelledBy="native-export-title" className="native-export-dialog" initialFocusRef={cancel} onClose={props.onClose}>
    <h2 id="native-export-title">{t("nativeExport.title")}</h2>
    <p>{t("nativeExport.contents")}</p>
    <p className="native-export-privacy">{t("nativeExport.privacy")}</p>
    <p>{t("nativeExport.scope")}</p>
    <p>{t("nativeExport.local")}</p>
    {!available&&<p role="alert">{t("nativeExport.changed")}</p>}
    <footer><button type="button" className="native-export-cancel" ref={cancel} onClick={props.onClose}>{t("nativeExport.cancel")}</button>
      <button type="button" className="primary-action" disabled={!available} onClick={download}>{t("nativeExport.download")}</button></footer>
  </Overlay>;
}
