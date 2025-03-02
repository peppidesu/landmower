import { useEffect, useMemo, useRef, useState } from "preact/hooks";
import axios from "axios";
import CopyButton from "./CopyButton";
import { useDebouncedCallback } from "use-debounce";
import spinner from "../assets/spinner.svg";

async function shorten(link: string, custom: boolean, customName: string): Promise<ShortenResult> {    
    let res = await axios.post(`${import.meta.env.VITE_SERVER_URL}api/links`, {
        link: link,
        key: custom ? customName : undefined
    }).catch(e => e.response);
    return res.status === 200 ? {
        success: true,
        data: res.data
    } : {
        success: false,
        error: res.data
    }
}
type ShortenResult = { success: true, data: any } | { success: false, error: any };

// Form validation ------------------------------

type ValidationResult = { valid: true } | { valid: false, reason: string };
type ValidateFormResponse = {
    link: ValidationResult,
    key?: ValidationResult
}



export function AddLinkForm() {        
    const form = useRef<HTMLFormElement>(null);

    const [link, setLink] = useState("");    
    const [customize, setCustomize] = useState(false); 
    const [key, setKey] = useState("");

    const [linkTouched, setLinkTouched] = useState(false);
    const [keyTouched, setKeyTouched] = useState(false);

    const [shortened, setShortened] = useState<ShortenResult | null>(null);    
    const [validateFormResponse, setValidateFormResponse] 
        = useState<ValidateFormResponse | null>(null);

    const [isValidating, setIsValidating] = useState(false);
    const validate = useDebouncedCallback(async (link, customize, key) => {        

        let res = await axios.post(`${import.meta.env.VITE_SERVER_URL}api/validate/add_form`, {
            link: link,
            key: customize ? key : undefined
        }).catch(e => e.response);

        if (res.status === 200) {
            setValidateFormResponse(res.data);
        }
        else {
            console.log("Error", res.data);
        }

        setIsValidating(false);
    }, 500);

    useEffect(() => {
        setIsValidating(true);
        validate(link, customize, key);
    }, [link, key, customize]);

    return (    
        <div class="max-w-2xl flex flex-col items-center px-8 gap-5 w-full">
            <h1 class="text-7xl">landmower</h1>
            <form ref={form} method="post" 
                onSubmit={async e => {
                    e.preventDefault();                             
                    let result = await shorten(link, customize, key);
                    if (result.success) {
                        form.current?.reset();
                    }
                    setShortened(result);
                }}
            >                        
                <input class={`w-full ${linkTouched && !validateFormResponse?.link.valid ? "invalid" : ""}`}
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
                    
                    <input class={`w-full ${keyTouched && !validateFormResponse?.key?.valid ? "invalid" : ""}`}
                        name="key" 
                        type="text" 
                        placeholder="Enter custom key"                     
                        onInput={e => setKey((e.target as HTMLInputElement).value)}                                                                                    
                        onBlur={_ => setKeyTouched(true)}
                        disabled={!customize}
                    />
                    <button 
                        class="pill h-10 w-40"
                        type="submit"
                        disabled={isValidating || !validateFormResponse?.link.valid || (customize && !validateFormResponse?.key?.valid)}
                    >
                        {isValidating ? <img class="m-auto" src={spinner}></img> : "Shorten"}
                    </button>  
                </div>                       
            </form>   

            {shortened && shortened.success
                ? <CopyButton text={`${import.meta.env.VITE_SERVER_URL}${shortened.data.key}`} />
            : null
            }  
        </div>
    )
}