import "./App.css";
import Navbar from "./components/Navbar";
import {listen} from "@tauri-apps/api/event";
import {invoke} from "@tauri-apps/api/tauri"
import { useState } from "react";

function App() {
  listen("discord", (payload) => {
    console.log(payload)
  })
  let data;
  invoke("getGuildById", {id: "560222296724471830"}).then((msg) => {data = msg; console.log(data);})
  invoke("greet", {name: "bozo"}).then((b) => {console.log(b)})
  return (
    <div className="App">
      <Navbar />
      <h1>
        {data}
      </h1>
    </div>
  );
}

export default App;
