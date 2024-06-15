import React, { useContext, useState } from "react";
import Header from "../sections/Header";
import TopDownHeading from "../elements/TopDownHeading";
import IconHolder from "../elements/IconHolder";
import attachment from "../../assests/attachment.svg";
import UniqueNumber from "../sections/UniqueNumber";
import { MainContext } from "../../context/MainContext";
import SelectSearchOption from "../elements/SelectSearchOption";

export default function CheckMintPage({ data }) {
  const {
    handlePage,
    contacts,
    setToast,
    showPopUp,
    showPopUpSecondary,
    handleRefresh,
  } = useContext(MainContext);

  const [dataForm, setDataForm] = useState({
    interest: "",
  });

  const changeHandle = (e) => {
    let value = e.target.value;
    let name = e.target.name;
    if (name === "interest") {
      let val = value.replace(/[^0-9/.]/g, "");
      setDataForm({ ...dataForm, [name]: val });
    } else {
      setDataForm({ ...dataForm, [name]: value });
    }
  };

  const handleSubmit = async () => {
    const form_data = new FormData();
    form_data.append("bill_name", data.name);
    form_data.append("interest", dataForm.interest);

    if (dataForm.interest) {
      await fetch("http://localhost:8000/bill/accept_mint", {
        method: "POST",
        body: form_data,
        mode: "cors",
      })
        .then((response) => {
          console.log(response);
          setToast(`Please Wait...`);
          if (response.status === 200) {
            showPopUpSecondary(false, "");
            showPopUp(false, "");
            handlePage("home");
            handleRefresh();
            setToast(`You have successfully approved minting for a bill.`);
          } else {
            setToast(`Something is wrong try again later.`);
          }
        })
        .catch((err) => err);
    } else {
      setToast(
        `Please Enter Percent`
      );
    }
  };

  const checkHandleSearch = (e) => {
    let value = e.target.value;
    let name = e.target.name;
    const isValidOption = contacts.some((d) => d.name == value);
    if (isValidOption || value === "") {
      setDataForm({ ...dataForm, [name]: value });
    } else {
      setDataForm({ ...dataForm, [name]: "" });
    }
  };
  console.log(dataForm);
  return (
    <div className="accept">
      <Header title="Check mint request" />
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
              <span className="detail">
                {data.currency_code} {data.amount_numbers}
              </span>
            </span>

            <span className="block mt">
              <label htmlFor="interest">interest (%)</label>
              <div className="form-input-row">
                <input
                  className="drop-shadow"
                  name="interest"
                  value={dataForm.interest}
                  onChange={changeHandle}
                  type="number"
                  placeholder="3"
                  required
                />
              </div>
            </span>

            <span className="block mt">
              <span className="accept-heading">Payer: </span>
              <span className="block detail">
                {data.drawee.name}, {data.place_of_drawing}
              </span>
            </span>
              {/*TODO:here must be who requested to mint*/}
          </div>
          <button className="btn mtt" onClick={handleSubmit}>
            ACCEPT
          </button>
            {/*TODO: decline button*/}
        </div>
      </div>
    </div>
  );
}
