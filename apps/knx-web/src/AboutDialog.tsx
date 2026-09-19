/** The About dialog: which application, which build, which licence, and what it never claims. */
// F5. Three facts, all of them checkable: the name, the build, the
// licence — plus the sentence this project has to keep saying out loud,
// that it is nobody's certified anything.
//
// The version is fetched rather than imported. `package.json`'s version is
// right here in the bundle and would have been one line, but it is the
// *frontend package's* version, not the build's, and nothing keeps the two
// in step. `GET /api/version` answers with what the running server was
// built from (`knx-server`'s `version_string`), commit and all, which is
// the only version worth putting in a bug report.
import { useEffect, useState } from "react";
import * as api from "./api";
import Overlay from "./Overlay";
import { useTranslate } from "./i18n";

export default function AboutDialog(props: { onClose: () => void }) {
  const { onClose } = props;
  const t = useTranslate();
  // `null` covers both "not answered yet" and "did not answer at all". The
  // dialog says so instead of inventing a number: a version the user is
  // going to paste into a bug report must never be a guess.
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .serverVersion()
      .then((v) => {
        if (!cancelled) setVersion(v.version);
      })
      .catch(() => {
        if (!cancelled) setVersion(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <Overlay labelledBy="about-dialog-title" className="about-dialog" onClose={onClose}>
      <h2 id="about-dialog-title">{t("about.title")}</h2>
      <dl className="about-dialog-facts">
        <dt>{t("about.version")}</dt>
        <dd className="about-dialog-version">{version ?? t("about.versionUnknown")}</dd>
        <dt>{t("about.licence")}</dt>
        <dd>{t("about.licenceValue")}</dd>
      </dl>
      <p>{t("about.independence")}</p>
      <p>{t("about.trademark")}</p>
      <footer className="about-dialog-footer">
        <button type="button" onClick={onClose}>
          {t("about.close")}
        </button>
      </footer>
    </Overlay>
  );
}
