import React, {useContext, useState} from "react";
import closeIcon from "../../assests/close-btn.svg";
import {MainContext} from "../../context/MainContext";

export default function EditContact({old_name}) {
    const {showPopUp, handleEditContact} = useContext(MainContext);
    const [contact, setContact] = useState({name: old_name});
    const handleChange = (e) => {
        setContact({...contact, [e.target.name]: e.target.value});
    };
    const handleSubmit = () => {
        handleEditContact(old_name, contact, showPopUp);
    };
    return (
        <div className="contact edit-contact">
            <div className="contact-head">
                <span className="contact-head-title">CHANGE CONTACT</span>
                <img onClick={() => showPopUp(false, "")} src={closeIcon}/>
            </div>
            <div className="contact-body">
                <input
                    type="text"
                    name="name"
                    id="name"
                    value={contact.name}
                    placeholder={old_name}
                    onChange={handleChange}
                />
                <input
                    type="text"
                    name="peer_id"
                    id="peer_id"
                    value={contact.peer_id}
                    placeholder="Node Identity"
                    onChange={handleChange}
                />
            </div>
            <button onClick={handleSubmit} className="btn">
                <span>UPDATE CONTACT</span>
            </button>
        </div>
    );
}
