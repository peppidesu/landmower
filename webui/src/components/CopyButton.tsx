import { useState } from "preact/hooks";

export default function CopyButton({ text }: { text: string }) {
    const [clicked, setClicked] = useState(false);

    return (
        <button class="text-gray-300 p-1 border-transparent border-b-1 hover:border-white transition-all duration-150 w-fit"
            onClick={() => {
                navigator.clipboard.writeText(text);                
                setClicked(true);
                setTimeout(() => setClicked(false), 2000);
            }}
        >
            {text}
            <i class={`ti pl-2 copy-icon
                ${clicked ? "copy-icon-animate" : ""}
            `}/>
        </button>
    );
}