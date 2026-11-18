import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import { App } from "./App";
import { api } from "./lib/api";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App api={api} />
  </React.StrictMode>,
);
