import React, {useContext, useEffect, useState} from "react";
import closeBtn from "../../assests/close-btn.svg";
import attachment from "../../assests/attachment.svg";
import {MainContext} from "../../context/MainContext";
import TopDownHeading from "../elements/TopDownHeading";
import IconHolder from "../elements/IconHolder";

import iconPay from "../../assests/pay.svg";
import iconAccept from "../../assests/accept.svg";
import iconEndorse from "../../assests/endorse.svg";
import iconRTA from "../../assests/reqToAccept.svg";
import iconRTP from "../../assests/reqToPay.svg";
import AcceptPage from "../pages/AcceptPage";
import RepayPage from "../pages/RepayPage";
import BuyPage from "../pages/BuyPage";
import EndorsePage from "../pages/EndorsePage";
import ReqAcceptPage from "../pages/ReqAcceptPage";
import ReqPaymentPage from "../pages/ReqPaymentPage";
import Key from "../Key";
import Bill from "../pages/Bill";
import SellPage from "../pages/SellPage";
import MintPage from "../pages/MintPage";
import CheckMintPage from "../pages/CheckMintPage";

export default function SingleBillDetail({ item }) {
  const { peer_id, showPopUp, showPopUpSecondary } = useContext(MainContext);
  const [singleBill, setSingleBill] = useState();

  //   const [singleBillChain, setSingleBillChain] = useState([]);

  useEffect(() => {
    fetch(`http://localhost:8000/bill/return/${item.name}`)
      .then((res) => res.json())
      .then((data) => {
        setSingleBill(data);
      })
      .catch((err) => {
        console.log(err.message);
      });
  }, []);

  //   useEffect(() => {
  //     fetch(`http://localhost:8000/bill/chain/return/${item.name}`)
  //       .then((res) => res.json())
  //       .then((data) => {
  //         console.log(data);
  //         setSingleBillChain(data.blocks);
  //       })
  //       .catch((err) => {
  //         console.log(err.message);
  //       });
  //   }, []);

  let payed = false;
  let accepted = false;
  let endorse = false;
  let sell = false;
  let check_mint = false;
  let mint = false;
  let buy = false;
  let req_to_pay = false;
  let req_to_acpt = false;
  let canMyPeerIdEndorse = peer_id == singleBill?.payee?.peer_id;
  let canMyPeerIdSell = peer_id == singleBill?.payee?.peer_id;
  //TODO add some logic
  let canMyPeerIdCheckMint = true;
  let canMyPeerIdMint = peer_id == singleBill?.payee?.peer_id;
  let canMyPeerIdBuy = peer_id == singleBill?.buyer?.peer_id;
  let canMyPeerIdAccept = peer_id == singleBill?.drawee?.peer_id;
  let canMyPeerIdPay = peer_id == singleBill?.drawee?.peer_id;
  let canMyPeerIdReqToAccept = peer_id == singleBill?.payee?.peer_id;
  let canMyPeerIdReqToPay = peer_id == singleBill?.payee?.peer_id;

  if (
    !singleBill?.payed &&
    !singleBill?.accepted &&
    !singleBill?.pending &&
    !singleBill?.waited_for_payment &&
    canMyPeerIdAccept
  ) {
    accepted = true;
  }
  if (
      !singleBill?.payed &&
      !singleBill?.pending &&
      !singleBill?.waited_for_payment &&
      canMyPeerIdEndorse
  ) {
      endorse = true;
  }
  if (
        !singleBill?.payed &&
        !singleBill?.pending &&
        !singleBill?.waited_for_payment &&
        canMyPeerIdSell
    ) {
      sell = true;
    }
    if (
        !singleBill?.payed &&
        !singleBill?.pending &&
        !singleBill?.waited_for_payment &&
        canMyPeerIdCheckMint
    ) {
        check_mint = true;
    }
    if (
        !singleBill?.payed &&
        !singleBill?.pending &&
        !singleBill?.waited_for_payment &&
        canMyPeerIdMint
    ) {
      mint = true;
    }
  if (
      !singleBill?.payed &&
      !singleBill?.pending &&
      singleBill?.waited_for_payment &&
      canMyPeerIdBuy
  ) {
      buy = true;
  }
  if (
    !singleBill?.payed &&
    !singleBill?.accepted &&
    !singleBill?.pending &&
    !singleBill?.requested_to_accept &&
    !singleBill?.waited_for_payment &&
    canMyPeerIdReqToAccept
  ) {
    req_to_acpt = true;
  }
  if (
    !singleBill?.payed &&
    !singleBill?.pending &&
    !singleBill?.requested_to_pay &&
    !singleBill?.waited_for_payment &&
    canMyPeerIdReqToPay
  ) {
    req_to_pay = true;
  }
  if (!singleBill?.payed &&
      !singleBill?.pending &&
      !singleBill?.waited_for_payment &&
      canMyPeerIdPay
  ) {
    payed = true;
  }

  const buttons = [
    { isVisible: payed, name: "PAY", icon: iconPay },
    { isVisible: accepted, name: "ACCEPT", icon: iconAccept },
    { isVisible: endorse, name: "ENDORSE", icon: iconEndorse },
      //todo icon mint
      {isVisible: mint, name: "MINT", icon: iconEndorse },
    //todo icon sell
    {isVisible: sell, name: "SELL", icon: iconEndorse },
      //todo icon check_mint
      {isVisible: check_mint, name: "CHECK_MINT", icon: iconEndorse },
      //todo icon buy
    {isVisible: buy, name: "BUY", icon: iconPay },
    {
      isVisible: req_to_acpt,
      name: "REQUEST TO ACCEPT",
      icon: iconRTA,
    },
    {
      isVisible: req_to_pay,
      name: "REQUEST TO PAY",
      icon: iconRTP,
    },
  ];

  const handlePageCalling = async (name) => {
    switch (name) {
      case "PAY":
        showPopUpSecondary(true, <RepayPage data={singleBill} />);
        break;
      case "BUY":
        showPopUpSecondary(true, <BuyPage data={singleBill} />);
        break;
      case "ACCEPT":
        showPopUpSecondary(true, <AcceptPage data={singleBill} />);
        break;
      case "ENDORSE":
        showPopUpSecondary(true, <EndorsePage data={singleBill} />);
        break;
      case "MINT":
            showPopUpSecondary(true, <MintPage data={singleBill} />);
            break;
      case "SELL":
        showPopUpSecondary(true, <SellPage data={singleBill}/>);
        break;
      case "CHECK_MINT":
        showPopUpSecondary(true, <CheckMintPage data={singleBill}/>);
        break;
      case "REQUEST TO ACCEPT":
        showPopUpSecondary(true, <ReqAcceptPage data={singleBill} />);
        break;
      case "REQUEST TO PAY":
        showPopUpSecondary(true, <ReqPaymentPage data={singleBill} />);
        break;
    }
  };
  const [seeMore, setSeeMore] = useState(false);
  let chain = singleBill?.chain_of_blocks?.blocks?.filter(
    (d) => d.operation_code === "Endorse"
  );
  let chainLength = singleBill?.chain_of_blocks?.blocks?.filter(
    (d) => d.operation_code === "Endorse"
  )?.length;
  if (seeMore) {
    chain = chain?.slice(0, chain?.length);
  } else {
    chain = chain?.slice(0, 3);
  }
  let showKey = singleBill?.requested_to_pay || singleBill?.payed;
  return (
    <>
      <div className="popup-head">
        <span className="popup-head-title">
          {item.place_of_drawing}, {item.date_of_issue}
        </span>
        <img
          className="close-btn"
          onClick={() => showPopUp(false, "")}
          src={closeBtn}
        />
      </div>
      <div className="popup-body">
        <div className="popup-body-inner">
          {singleBill && (
            <span
              onClick={() => {
                showPopUpSecondary(true, <Bill data={singleBill} />);
              }}
              className="download"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 16 18"
                fill="none"
              >
                <path
                  d="M9.5 2.375C9.5 1.54657 8.82843 0.875 8 0.875C7.17157 0.875 6.5 1.54657 6.5 2.375L6.5 9.57474L4.182 7.25674C3.59621 6.67095 2.64646 6.67095 2.06068 7.25674C1.47489 7.84252 1.47489 8.79227 2.06068 9.37806L6.5 13.8174V14.125H1.5C0.671573 14.125 0 14.7966 0 15.625C0 16.4534 0.671573 17.125 1.5 17.125H14.5C15.3284 17.125 16 16.4534 16 15.625C16 14.7966 15.3284 14.125 14.5 14.125H9.5V13.9387L14.1317 9.30699C14.7175 8.7212 14.7175 7.77145 14.1317 7.18567C13.5459 6.59988 12.5962 6.59988 12.0104 7.18567L9.5 9.69608L9.5 2.375Z"
                  fill="white"
                />
              </svg>
              download bill
            </span>
          )}
          {showKey && (
            <Key
              payed={singleBill?.payed}
              peerId={peer_id}
              payee={singleBill?.payee}
              privatekey={singleBill?.pr_key_bill}
              pending={singleBill?.pending}
              confirmations={singleBill?.number_of_confirmations}
            />
          )}
          <div className="head">
            <TopDownHeading upper="Against this" lower="Bill Of Exchange" />
            <IconHolder icon={attachment} />
          </div>
          <div className="block mt">
            <span className="block">
              <span className="accept-heading">pay on </span>
              <span className="detail">{singleBill?.maturity_date}</span>
            </span>
            <span className="block">
              <span className="accept-heading">to the order of </span>
              <span className="block detail ">{singleBill?.payee.name}</span>
            </span>
            <span className="block">
              <span className="accept-heading">the sum of </span>
              <span className="detail">
                {singleBill?.currency_code} {singleBill?.amount_numbers}
              </span>
            </span>
            <span className="block mt">
              <span className="accept-heading">Payer: </span>
              <span className="block detail">{singleBill?.drawee.name}</span>
            </span>
            {chain?.length > 0 && (
              <span className="block mt">
                <span className="accept-heading">Endorsed by: </span>
                <span className="block detail fs-small">
                  {chain?.map((d, i) => (
                    <li key={i}>{d.label}</li>
                  ))}
                  {chainLength > 3 && (
                    <span
                      className="see-more-btn"
                      onClick={() => setSeeMore(!seeMore)}
                    >
                      {!seeMore ? "see more" : "see less"}
                    </span>
                  )}
                </span>
              </span>
            )}
          </div>
        </div>
        <div className="popup-btns">
          {buttons.map(({ isVisible, name, icon }, index) => {
            if (isVisible) {
              return (
                <button
                  key={index}
                  onClick={() => handlePageCalling(name)}
                  className="popup-btns-btn"
                >
                  <img src={icon} /> <span>{name}</span>
                </button>
              );
            }
          })}
        </div>
      </div>
    </>
  );
}
