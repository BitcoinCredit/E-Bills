import React, { useContext } from "react";
import IconHolder from "../elements/IconHolder";
import SecondaryIcon from "../elements/SecondaryIcon";

import setting from "../../assests/setting.svg";
import contact from "../../assests/contact.svg";
import payment_options from "../../assests/payment-options.svg";
import payment_channel from "../../assests/payment-chanel.svg";
import profile from "../../assests/profile.svg";
import { MainContext } from "../../context/MainContext";

export default function HomeHeader() {
  const { handlePage, identity } = useContext(MainContext);
  let name = identity?.name?.split(" ");

  let first = name[0];
  let second = name[name?.length - 1];
  let initials = first[0] + second[0];
  return (
    <div className="home-header">
      <div className="home-header-left">
        <IconHolder handleClick={() => handlePage("setting")} icon={setting} />
        <IconHolder handleClick={() => handlePage("contact")} icon={contact} />
        <IconHolder handleClick={() => handlePage("findbill")} icon={contact} />
        {/* <SecondaryIcon
          routing={() => handlePage("contact")}
          iconImage={contact}
        />{" "} */}
      </div>
      <div className="home-header-right">
        {/* <SecondaryIcon
          routing={() => handlePage("dont")}
          iconImage={payment_channel}
        /> */}
        <IconHolder
          handleClick={() => handlePage("dont")}
          icon={payment_channel}
        />
        <span
          className={"icon-container profile-icon"}
          onClick={() => handlePage("identity")}
        >
          <span className={"icon"}>{initials}</span>
        </span>
      </div>
    </div>
  );
}
