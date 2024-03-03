import React from "react";
import ReactDOM from "react-dom/client";
import "./index.scss";
import App from "./App";
import {MainProvider} from "./context/MainContext";
import {DevSupport} from "@react-buddy/ide-toolbox";
import {ComponentPreviews, useInitial} from "./dev";

const root = ReactDOM.createRoot(document.getElementById("root"));

root.render(
    <MainProvider>
        <DevSupport ComponentPreviews={ComponentPreviews}
                    useInitialHook={useInitial}
        >
            <App/>
        </DevSupport>
    </MainProvider>
);
