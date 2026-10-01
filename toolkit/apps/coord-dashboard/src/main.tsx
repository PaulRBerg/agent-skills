import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "@/app";
import { configureHomeDirectory } from "@/lib/format";
import "@/styles.css";

configureHomeDirectory(
  document.querySelector('meta[name="dashboard-home"]')?.getAttribute("content") ??
    undefined,
);

const rootElement = document.getElementById("root");
if (!(rootElement instanceof HTMLElement)) {
  throw new Error("Dashboard root element is missing");
}

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
