import React, { useContext } from "react";
import { MainContext } from "../../context/MainContext";

export default function MintPage({ data }) {
    const { handlePage, contacts, showPopUp, showPopUpSecondary, handleRefresh } =
        useContext(MainContext);

    const handleSubmit = async () => {
            fetch(`http://localhost:8000/bill/mint/${data.name}`)
                .then((response) => {
                    console.log(response);
                    showPopUpSecondary(false, "");
                    showPopUp(false, "");
                    handlePage("home");
                    handleRefresh();})
                .catch((err) => {
                    console.log(err.message);
                });
    }

    return (
        <button
            onClick={handleSubmit}
            className="home-container-bills-container"
        >MINT</button>
    );
}