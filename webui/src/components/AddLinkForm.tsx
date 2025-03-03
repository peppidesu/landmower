import { useEffect, useMemo, useRef, useState } from "preact/hooks";
import axios from "axios";
import CopyButton from "./CopyButton";
import { useDebouncedCallback } from "use-debounce";
import Spinner from "../assets/spinner.svg?react";

type Jsend<T, F> = { status: "success", data: T }
                 | { status: "fail", data: F }
                 | { status: "error", data: string };

type Entry = { 
    link: string, 
    key: string, 
    metadata: { 
        used: number, 
        last_used: string, created: string 
    } 
}

type AddLinkResponse = Jsend<AddLinkSuccessData, AddLinkFailData>;
type ValidateResponse = Jsend<null, AddLinkFailData>;
type AddLinkSuccessData = { key: string, entry: Entry };
type AddLinkFailData = { link?: string, key?: string };

async function addLink(link: string, custom: boolean, customName: string): Promise<AddLinkResponse> {    
    let res = await axios.post(`${import.meta.env.VITE_SERVER_URL}api/links`, {
        link: link,
        key: custom ? customName : undefined
    });
    return res.data;
}


export function AddLinkForm() {        
    const form = useRef<HTMLFormElement>(null);

    const [link, setLink] = useState("");    
    const [customize, setCustomize] = useState(false); 
    const [key, setKey] = useState("");

    const [linkTouched, setLinkTouched] = useState(false);
    const [keyTouched, setKeyTouched] = useState(false);

    const [addLinkRes, setShortenRes] = useState<AddLinkResponse | null>(null);    
    const [validateRes, setValidateRes] = useState<ValidateResponse | null>(null);

    const [isValidating, setIsValidating] = useState(false);
    const validate = useDebouncedCallback(async (link, customize, key) => {
        let res = await axios.post(`${import.meta.env.VITE_SERVER_URL}api/validate/add_link`, {
            link: link,
            key: customize ? key : undefined
        });
        setValidateRes(res.data);
        setIsValidating(false);
    }, 500);
    
    useEffect(() => {
        setIsValidating(true);
        validate(link, customize, key);
    }, [link, key, customize]);

    const invalid = validateRes?.status === "fail" ? validateRes.data : null;

    return (    
        <div class="max-w-2xl flex flex-col items-center px-8 gap-5 w-full">
            <h1 class="text-7xl">landmower</h1>
            <form ref={form} method="post" 
                onSubmit={async e => {
                    e.preventDefault();                             
                    let result = await addLink(link, customize, key);
                    if (result.status === "success") {
                        form.current?.reset();
                    }
                    setShortenRes(result);
                }}
            >                        
                <input class={`w-full ${linkTouched && invalid?.link ? "invalid" : ""}`}
                    name="link" 
                    type="text"                  
                    onInput={e => setLink((e.target as HTMLInputElement).value)}
                    onBlur={_ => setLinkTouched(true)}
                    placeholder="Your very, very long URL" 
                />
                <div class="flex items-center gap-2 pt-2 w-full">                               
                    <label for="customize" class="grid grid-cols-[1em_auto] px-2 gap-4 items-center text-gray-300">
                        <input type="checkbox"id="customize" name="customize" 
                            onClick={() => setCustomize(!customize)}
                            class="row-start-1 row-end-1 col-start-1 col-end-1 w-0 h-0"
                        />                    
                        Customize
                    </label>
                    
                    <input class={`w-full ${keyTouched && invalid?.key ? "invalid" : ""}`}
                        name="key" 
                        type="text" 
                        placeholder="Enter custom key"                     
                        onInput={e => setKey((e.target as HTMLInputElement).value)}                                                                                    
                        onBlur={_ => setKeyTouched(true)}
                        disabled={!customize}
                    />
                    <button 
                        class="pill h-10 w-40 "
                        type="submit"
                        disabled={isValidating || !!invalid}
                    >
                        {isValidating ? <Spinner class="m-auto" /> : "Shorten"}
                    </button>  
                </div>
                <ul class="text-red-400 text-sm h-[2rlh] w-full text-right pt-2 pr-1">
                    {linkTouched && invalid?.link && <li>{invalid?.link}</li>}
                    {keyTouched  && invalid?.key && <li>{invalid?.key}</li>}
                </ul>
                
                                          
            </form>   

            {addLinkRes && addLinkRes.status === "success" &&
                <CopyButton text={`${import.meta.env.VITE_SERVER_URL}${addLinkRes.data.key}`} />                
            }
        </div>
    )
}