import React, { useContext, useEffect, useState } from "react";
import closeIcon from "../../assests/close-btn.svg";
import SingleContact from "../SingleContact";

import { MainContext } from "../../context/MainContext";
import AddContact from "../popups/AddContact";

export default function FindBill() {
  const { handlePage } =
    useContext(MainContext);
  const [search, setSearch] = useState("");
  const handleSearchChange = (e) => {
    setSearch(e.target.value);
  };
    const handleSubmit = () => {
        fetch("http://localhost:8000/bill/find/" + search, {
            mode: "cors",
        }).then(response =>  console.log(response));
    };
  return (
    <div className="contact">
      <div className="contact-head">
        <span className="contact-head-title">FIND BILL</span>
        <img
          className="close-btn"
          onClick={() => {
            handlePage("home");
          }}
          src={closeIcon}
        />
      </div>
      <input
        type="text"
        className="input-contact"
        placeholder="Search Bill"
        onChange={handleSearchChange}
      />
      <button onClick={handleSubmit} className="btn">
        FIND
      </button>
    </div>
  );
}
