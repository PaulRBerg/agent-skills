import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createBrowserRouter } from "react-router";
import { RouterProvider } from "react-router/dom";

import { AppErrorBoundary } from "@/app-error-boundary.js";
import { App } from "@/app.js";
import { configureHomeDirectory } from "@/lib/format.js";

import "@/styles.css";

configureHomeDirectory(
  document.querySelector('meta[name="dashboard-home"]')?.getAttribute("content") ?? undefined
);

const rootElement = document.querySelector("#root");
if (!(rootElement instanceof HTMLElement)) {
  throw new Error("Dashboard root element is missing");
}

const router = createBrowserRouter([
  { Component: App, ErrorBoundary: AppErrorBoundary, path: "*" },
]);

createRoot(rootElement).render(
  <StrictMode>
    <RouterProvider router={router} />
  </StrictMode>
);
