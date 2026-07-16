import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "pretendard/dist/web/variable/pretendardvariable-dynamic-subset.css";
import "./styles/tokens.css";
import "./styles/typeset.css";
import "./styles/primitives.css";
import "./styles.css";
import "./styles/theme.css";
import "./styles/layout.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
