import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { api } from "./lib/api";
import "./index.css";

/** Erros fora do React também vão para o log do app. */
function reportGlobalErrors() {
  window.addEventListener("error", (e) => {
    api.uiError(`Erro de script: ${e.message} (${e.filename}:${e.lineno})`).catch(() => undefined);
  });
  window.addEventListener("unhandledrejection", (e) => {
    const reason = e.reason instanceof Error ? e.reason.message : String(e.reason);
    api.uiError(`Promessa rejeitada: ${reason}`).catch(() => undefined);
  });
}

/** Sinal de vida para o vigia do núcleo, que recarrega a tela se ela travar. */
function startHeartbeat() {
  const beat = () => api.uiHeartbeat().catch(() => undefined);
  beat();
  setInterval(beat, 1000);
}

async function boot() {
  // Simulador do núcleo, só em desenvolvimento no navegador (?mock).
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window) && location.search.includes("mock")) {
    await import("./dev/mockTauri");
  }
  reportGlobalErrors();
  startHeartbeat();
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <ErrorBoundary>
        <App />
      </ErrorBoundary>
    </React.StrictMode>,
  );
}

void boot();
