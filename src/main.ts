import "./styles.css";
import { SagewatchApp } from "./App";

const root = document.querySelector<HTMLElement>("#app");
if (!root) throw new Error("Sagewatch app root was not found");
void new SagewatchApp(root).start();
