import React, { useContext, useEffect } from "react";
import IconHolder from "./IconHolder";
import { MainContext } from "../../context/MainContext";
import SingleBillDetail from "../popups/SingleBillDetail";

const signCalculation = (peer_id, items) => {
  if (peer_id == items.drawee.peer_id) {
    //   name = `${items?.drawee?.name} has to pay ${items?.payee?.name}`;
    return "-";
  } else if (peer_id == items.drawer.peer_id) {
    //   name = `${items.drawee.name} ${items.payee.name}`;
    return "x";
  } else if (peer_id == items.payee.peer_id) {
    //   name = `${items.drawee.name} ${items.payee.name}`;
    return "+";
  } else if (peer_id == items.endorsee.peer_id) {
    //   name = `${items.drawee.name} ${items.payee.name}`;
    return "+";
  }
};
const namehandling = (peer_id, items) => {
  if (peer_id == items?.drawee?.peer_id) {
    return items?.payee?.name;
  } else if (peer_id == items?.drawee?.peer_id) {
    return items?.drawee?.name;
  } else {
    return items?.drawee?.name;
  }
};

function BillDetails({ data, icon, filter }) {
  const { peer_id, showPopUp } = useContext(MainContext);
  var filteredData = data;
  if (filter?.imPayee) {
    filteredData = data.filter((d) => d.payee.peer_id === peer_id);
  } else if (filter?.imDrawee) {
    filteredData = data.filter((d) => d.drawee.peer_id === peer_id);
  } else if (filter?.imDrawer) {
    filteredData = data.filter((d) => d.drawer.peer_id === peer_id);
  }
  return (
    <>
      {filteredData.map((items, i) => {
        let sign = signCalculation(peer_id, items);
        let name = namehandling(peer_id, items);
        return (
          <div
            key={i}
            onClick={() => showPopUp(true, <SingleBillDetail item={items} />)}
            className="home-container-bills-container"
          >
            <IconHolder icon={icon} />
            <div className="details">
              <span className="name">{name}</span>
              <span className="date">{items.date_of_issue}</span>
            </div>
            <div className="currency-details">
              <div
                className={
                  sign === "+"
                    ? "amount"
                    : sign === "x"
                    ? "amount grey"
                    : sign === "-"
                    ? "amount red"
                    : "amount grey"
                }
              >
                <span>{sign === "x" ? "" : sign}</span>
                <span>{items.amount_numbers}</span>
              </div>
              <span className="currency">{items.currency_code}</span>
            </div>
          </div>
        );
      })}
    </>
  );
}

export default BillDetails;
