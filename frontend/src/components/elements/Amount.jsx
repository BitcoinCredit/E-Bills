import React from "react";
import Arrow from "../elements/Arrow";

export default function Amount({ color, amount, currency, degree }) {
  return (
    <div className="home-container-amount-single ">
      <Arrow color={color} rotate={degree} />
      <span style={{ textTransform: "uppercase", color: `#${color}` }}>
        {amount} <span className="currency">{currency}</span>
      </span>
    </div>
  );
}
