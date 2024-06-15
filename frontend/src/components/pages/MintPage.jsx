import React, { useContext, useState } from "react";
import Header from "../sections/Header";
import TopDownHeading from "../elements/TopDownHeading";
import IconHolder from "../elements/IconHolder";
import attachment from "../../assests/attachment.svg";
import UniqueNumber from "../sections/UniqueNumber";
import { MainContext } from "../../context/MainContext";
import SelectSearchOption from "../elements/SelectSearchOption";

export default function MintPage({ data }) {
    const { handlePage, contacts, showPopUp, showPopUpSecondary, handleRefresh } =
        useContext(MainContext);

    const [billMint, setBillMint] = useState("");
    const changeHandle = (e) => {
        setBillMint(e.target.value);
    };
    const handleSubmit = async () => {
        const form_data = new FormData();
        form_data.append("bill_name", data.name);
        form_data.append("mint_node", billMint);
        await fetch("http://localhost:8000/bill/request_to_mint", {
            method: "POST",
            body: form_data,
            mode: "cors",
        })
            .then((response) => {
                console.log(response);
                showPopUpSecondary(false, "");
                showPopUp(false, "");
                handlePage("home");
                handleRefresh();
            })
            .catch((err) => err);
    };

    const checkHandleSearch = (e) => {
        let value = e.target.value;
        let name = e.target.name;
        const isValidOption = contacts.some((d) => d.name == value);
        if (isValidOption || value === "") {
            setBillMint(e.target.value);
        } else {
            setBillMint("");
        }
    };
    return (
        <div className="accept">
            <Header title="Mint" />
            <UniqueNumber UID={data.place_of_payment} date={data.date_of_issue} />
            <div className="head">
                <TopDownHeading upper="Against this" lower="Bill Of Exchange" />
                <IconHolder icon={attachment} />
            </div>
            <div className="accept-container">
                <div className="accept-container-content">
                    <div className="block mt">
            <span className="block">
              <span className="accept-heading">pay on </span>
              <span className="detail">{data.date_of_issue}</span>
            </span>
                        <span className="block">
              <span className="accept-heading">the sum of </span>
              <span className="detail" style={{ textTransform: "uppercase" }}>
                {data.currency_code} {data.amount_numbers}
              </span>
            </span>
                        <span className="block mt">
              <span className="accept-heading">to the order of </span>
              <span className="block detail search-select">
                <SelectSearchOption
                    identity="drawee_name"
                    placingHolder="Select Your Mint"
                    classs="endorse-select"
                    valuee={billMint}
                    changeHandle={changeHandle}
                    checkHandleSearch={checkHandleSearch}
                    options={contacts}
                />
              </span>
            </span>
                        <span className="block mt">
              <span className="accept-heading">Payer: </span>
              <span className="block detail">
                {data.drawee.name}, {data.place_of_drawing}
              </span>
            </span>
                        <span className="block mt">
              <span className="accept-heading">Minted by: </span>
              <span className="block detail">
                {data.payee.name}, {data.place_of_payment}
              </span>
            </span>
                    </div>
                    <button className="btn mtt" onClick={handleSubmit}>
                        SIGN
                    </button>
                </div>
            </div>
        </div>
    );
}
