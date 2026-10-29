import { createRoot } from "react-dom/client";
import "./osd.css";
import { Overlay } from "./Overlay";

createRoot(document.getElementById("root")!).render(<Overlay />);
