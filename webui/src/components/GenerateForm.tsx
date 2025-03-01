import { useEffect, useRef, useState } from "preact/hooks";
import axios from "axios";

async function shorten(url: string, custom: boolean, customName: string): Promise<ShortenResult> {    
    let res = await axios.post(`${import.meta.env.VITE_SERVER_URL}api/links`, {
        link: url,
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

function useStateDebounced<T>(initial: T, delay: number) {
    const [value, setValue] = useState(initial);
    const [debounced, setDebounced] = useState(initial);
    useEffect(() => {
        const timeout = setTimeout(() => setDebounced(value), delay);
        return () => clearTimeout(timeout);
    }, [value, delay]);
    return [debounced, setValue, setDebounced] as const;
}

// Form validation ------------------------------

type ValidationResult = { valid: true } | { valid: false, message: string };
type ShortenResult = { success: true, data: any } | { success: false, error: any };

async function validateLink(url: string): Promise<ValidationResult> {
    if (url.length === 0) 
        return { valid: false, message: "URL cannot be empty" };

    const url_regex = /[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)/;
    if (!url_regex.test(url)) 
        return { valid: false, message: "Invalid URL" };

    return { valid: true };
}

async function validateKey(key: string): Promise<ValidationResult> {
    if (key.length < 3) 
        return { 
            valid: false, 
            message: "Key needs to be at least 3 characters" 
        };

    if (!/^[a-z0-9\-]+$/.test(key)) 
        return { 
            valid: false, 
            message: "Key can only contain lowercase letters, numbers and hyphens (-)" 
        };
    
    let res = await axios.get(`${import.meta.env.VITE_SERVER_URL}api/links/${key}`)
        .catch(e => e.response);
    if (res.status !== 404) 
        return { 
            valid: false, 
            message: "Key already in use" 
        };
    
    return { valid: true };
}

// ----------------------------------------------

export function GenerateForm() {        
    const form = useRef<HTMLFormElement>(null);

    const [customize, setCustomize] = useState(false); 

    const [linkDebounced, setLink] = useStateDebounced("", 100);    
    const [keyDebounced, setKey] = useStateDebounced("", 100);

    const [linkTouched, setLinkTouched] = useState(false);
    const [keyTouched, setKeyTouched] = useState(false);
        
    const [linkValid, setLinkValid] = useState({ valid: true } as ValidationResult);

    const [shortened, setShortened] = useState<ShortenResult | null>(null);
    useEffect(() => {
        validateLink(linkDebounced)
            .then(result => setLinkValid(result));
    }, [linkDebounced]);

    const [keyValid, setKeyValid] = useState({ valid: true } as ValidationResult);
    useEffect(() => {        
        validateKey(keyDebounced)
            .then(result => setKeyValid(result));
    }, [keyDebounced]);    

    const allFieldsValid = linkValid.valid && keyValid.valid;    
    
    console.log(linkValid.valid)

    return (    
        <div class="max-w-2xl flex flex-col items-center px-8 gap-5 w-full">
            <h1 class="text-7xl">landmower</h1>
            <form ref={form} method="post" 
                onSubmit={async e => {
                    e.preventDefault();                             
                    let result = await shorten(linkDebounced, customize, keyDebounced);
                    if (result.success) {
                        form.current?.reset();
                    }
                    else {
                        console.log("Error", result.error);
                    }
                    setShortened(result);
                }}
            >                        
                <input class={`w-full ${(linkTouched && !linkValid.valid) ? "invalid" : ""}`}
                    name="link" 
                    type="text"                  
                    onInput={e => setLink((e.target as HTMLInputElement).value)}
                    onBlur={() => setLinkTouched(true)}
                    placeholder="Your very, very long URL" 
                />
                <div class="flex items-center gap-2 pt-2 w-full">                               
                    <label for="customize" class="grid grid-cols-[1em_auto] px-2 gap-4 items-center text-gray-300">
                        <input type="checkbox"id="customize" name="customize" 
                            onClick={() => {
                                setCustomize(!customize);
                                if (!customize) setKeyTouched(false);
                            }}
                            class="row-start-1 row-end-1 col-start-1 col-end-1 w-0 h-0"
                        />                    
                        Customize
                    </label>
                    
                    <input class={`w-full ${(keyTouched && customize && !keyValid.valid) ? "invalid" : ""}`}
                        name="key" 
                        type="text" 
                        placeholder="Enter custom key"                     
                        onInput={e => setKey((e.target as HTMLInputElement).value)}                                                            
                        onBlur={() => setKeyTouched(true)}
                        disabled={!customize}
                    />
                    <button class="pill" 
                        type="submit"
                        disabled={!allFieldsValid}
                    >
                        Shorten
                    </button>  
                </div>       
                <ul class="text-red-400 text-right pr-2 pt-2 text-sm h-[2rlh]">
                    {linkTouched && !linkValid.valid && <li>{linkValid.message}</li>}
                    {keyTouched && customize && !keyValid.valid && <li>{keyValid.message}</li>}
                </ul>                
            </form>   

            {shortened && shortened.success
                ? <button class="text-gray-300 p-1 border-transparent border-b-1 hover:border-white transition-all duration-150 w-fit">
                    {import.meta.env.VITE_SERVER_URL}{shortened.data.key}
                    <i class="ti ti-copy pl-2"/>
                </button>
            : null
            }  
        </div>
    )
}