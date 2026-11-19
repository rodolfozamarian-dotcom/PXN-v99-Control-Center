import { PageHeader, Panel } from "../components/ui";
import { useT } from "../lib/i18n";
import { APP_VERSION } from "../lib/version";

export function About() {
  const t = useT();
  return (
    <div>
      <PageHeader code="07" title={t("nav.about")} subtitle={`PXN V99 Control Center ${APP_VERSION}`} />
      <div className="grid grid-cols-12 gap-4">
        <Panel code="A" title={t("about.how")} className="col-span-8">
          <ul className="space-y-3 leading-relaxed text-steel">
            {(["about.p1", "about.p2", "about.p3", "about.p4"] as const).map((k) => (
              <li key={k} className="flex gap-3">
                <span className="mono mt-[3px] text-[10px] text-accent">▸</span>
                <span>{t(k)}</span>
              </li>
            ))}
          </ul>
        </Panel>
      </div>
    </div>
  );
}
