// Se uma tela falhar ao desenhar, mostra o erro e um botão para recarregar
// (em vez de deixar a janela vazia) e registra o erro no log do app.
import { Component, type ReactNode } from "react";
import { api } from "../lib/api";
import { translate } from "../lib/i18n";
import { settingsStore } from "../lib/store";

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<{ children: ReactNode }, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: { componentStack?: string | null }) {
    api.uiError(`Erro de tela: ${error.message}\n${(info.componentStack ?? "").slice(0, 1200)}`).catch(() => undefined);
  }

  render() {
    if (!this.state.error) return this.props.children;
    const t = (k: Parameters<typeof translate>[1]) => translate(settingsStore.get().lang, k);
    return (
      <div className="panel mx-auto mt-16 max-w-xl p-6">
        <div className="label mb-2 text-bad">{t("ui.crashTitle")}</div>
        <p className="text-steel">{t("ui.crashText")}</p>
        <p className="selectable mono mt-3 break-all text-xs text-dim">{this.state.error.message}</p>
        <button
          onClick={() => location.reload()}
          className="mono mt-5 h-8 border border-accent bg-accent px-3.5 text-[11px] font-semibold uppercase tracking-[0.12em] text-bg"
        >
          {t("ui.reload")}
        </button>
      </div>
    );
  }
}
