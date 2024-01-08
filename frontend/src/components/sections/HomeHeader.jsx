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
  const { handlePage } = useContext(MainContext);
  return (
    <div className="home-header">
      <div className="home-header-left">
        <IconHolder handleClick={() => handlePage("setting")} icon={setting} />
        <IconHolder handleClick={() => handlePage("contact")} icon={contact} />
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
        <IconHolder handleClick={() => handlePage("identity")} icon={profile} />
      </div>
    </div>
  );
}
