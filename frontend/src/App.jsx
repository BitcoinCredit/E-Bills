import React, { useContext, useEffect, useState } from "react";

import IssuePage from "./components/pages/IssuePage";
import { MainContext } from "./context/MainContext";
import HomePage from "./components/pages/HomePage";
import ActivityPage from "./components/pages/ActivityPage";
import IdentityPage from "./components/pages/IdentityPage";
import SettingPage from "./components/pages/SettingPage";
import ErrrorPage from "./components/pages/ErrrorPage";
import Contact from "./components/pages/Contact";
import FindBill from "./components/pages/FindBill";

export default function App() {
  const { toast, loading, current, popUp, popUp2 } = useContext(MainContext);
  // Set data for bill issue
  const [operation_codes, setOperationCodes] = useState([]);
  // Set operation codes
  useEffect(() => {
    fetch("http://localhost:8000/opcodes/return", {
      mode: "cors",
    })
      .then((res) => res.json())
      .then((data) => {
        setOperationCodes(data);
      })
      .catch((err) => {
        console.log(err.message);
      });
  }, []);

  const activePage = () => {
    switch (current) {
      case "identity":
        return <IdentityPage />;
      case "home":
        return <HomePage />;
      case "activity":
        return <ActivityPage />;
      case "contact":
        return <Contact />;
      case "findbill":
            return <FindBill />;
      case "issue":
        return <IssuePage />;
      case "setting":
        return <SettingPage />;
      default:
        return <HomePage />;
    }
  };
  //identity if this empty
  if (loading) {
    return (
      <div className="loading-main">
        <div className="loading">
          <div className="loading-sub">
            <div></div>
          </div>
        </div>
      </div>
    );
  } else {
    return (
      <>
        {toast && <span className="toast">{toast}</span>}
        {popUp.show && <div className="popup">{popUp.content}</div>}
        {popUp2.show && <div className="popup">{popUp2.content}</div>}
        {activePage()}
      </>
    );
  }
}
