import React, {useState} from "react";
import {useContext} from "react";
import closeIcon from "../../assests/close-btn.svg";
import addIcon from "../../assests/add.svg";
import {MainContext} from "../../context/MainContext";

export default function AddContact() {
    const {showPopUp, handleAddContact} = useContext(MainContext);
    const [contact, setContact] = useState({name: "", peer_id: ""});
    const handleChange = (e) => {
        setContact({...contact, [e.target.name]: e.target.value});
    };
    const handleSubmit = () => {
        handleAddContact(contact, showPopUp);
    };
    return (
        <div className="contact add-contact">
            <div className="contact-head">
                <span className="contact-head-title">CONTACT</span>
                <img onClick={() => showPopUp(false, "")} src={closeIcon}/>
            </div>
            <div className="contact-body">
                <input
                    type="text"
                    name="name"
                    id="name"
                    value={contact.name}
                    placeholder="Full Name"
                    onChange={handleChange}
                />
            </div>
            <button onClick={handleSubmit} className="btn">
                <img src={addIcon}/>
                <span>ADD CONTACT</span>
            </button>
        </div>
    );
}
