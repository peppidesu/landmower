import { useEffect, useState } from "preact/hooks";
import { useDebouncedCallback } from "use-debounce";
import { delete_link, Entry, get_links, GetLinksResponse } from "../api"

export default function ManageLinks() {
    const [links, setLinks] = useState<GetLinksResponse | null>(null);
    
    const [confirmationVisible, setConfirmationVisible] = useState(false);
    const [entryToDelete, setEntryToDelete] = useState<Entry | null>(null);

    const handleDelete = (entry: Entry) => {
        setEntryToDelete(entry);
        setConfirmationVisible(true);
    }

    const fetch = useDebouncedCallback(async () => {
        let res = await get_links();
        setLinks(res);
    }, 500);

    useEffect(() => {
        fetch();
    }, []);

    const openDeleteDialog = (entry: Entry) => {
        setEntryToDelete(entry);
        setConfirmationVisible(true);
    }

    return (
        <>
        <div class="h-full max-w-2xl w-full flex m-2 px-3 flex-col gap-2 overflow-y-scroll overflow-x-hidden">
            {links?.status === "success" 
                ? links.data.length === 0
                    ? <div class="text-center text-lg">No links found</div>
                    : links.data.map(entry => (
                        <LinkRow entry={entry} onDelete={() => openDeleteDialog(entry)} />
                    ))
                : <div class="text-center text-lg">Loading...</div>
            }
        </div>
        <ConfirmationDialog
            visible={confirmationVisible}
            entry={entryToDelete}
            onCancel={() => setConfirmationVisible(false)}
            onDelete={async () => {
                await delete_link(entryToDelete!.key);
                fetch();
                setConfirmationVisible(false);
            }}
        />
        </>
    );
}

type LinkRowProps = {
    entry: Entry,
    onDelete: () => void
}
function LinkRow({entry, onDelete}: LinkRowProps) {
    const created = new Date(entry.metadata.created).toLocaleDateString();
    const lastUsed = new Date(entry.metadata.last_used).toLocaleDateString();
    return (
        <div class="flex flex-col p-3 bg-gray-800/50 rounded-md">
            <div class="flex items-center text-lg">                
                <span class="overflow-hidden overflow-ellipsis text-nowrap">                    
                    <span class="text-gray-400">{import.meta.env.VITE_SERVER_URL}</span>
                    <b>{entry.key}</b>
                </span>                
                <button onClick={onDelete} class="ml-auto grid place-items-center aspect-square h-full delete-button">
                    <i class="ti ti-trash col-span-full row-span-full"></i>
                </button>                
            </div>
            <div class="overflow-hidden w-full">{entry.link}</div>
            <div class="text-gray-400 text-sm ml-5">Used {entry.metadata.used} times</div>
            <div class="text-gray-400 text-sm ml-5">Created {created}</div>
            <div class="text-gray-400 text-sm ml-5">Last used {lastUsed}</div>
        </div>
    );
}

type ConfirmationDialogProps = {
    entry: Entry | null
    visible: boolean
    onCancel: () => void
    onDelete: () => void
}
function ConfirmationDialog({visible, entry, onCancel, onDelete}: ConfirmationDialogProps) {    
    return (
        <div class={`dialog-wrapper ${visible ? '' : 'hidden'}`}>
            <div class="bg-gray-800 mx-4 p-5 rounded-md max-w-lg">
                <div class="text-lg">Are you sure you want to delete this link?</div>
                <div class="text-gray-300 mt-2 text-center break-all">{import.meta.env.VITE_SERVER_URL}{entry?.key ?? ""}</div>                
                <div class="flex gap-5 mt-4">
                    <button class="pill w-full btn-danger" onClick={onDelete}>Delete</button>
                    <button class="pill w-full btn-primary" onClick={onCancel}>Cancel</button>
                </div>
            </div>
        </div>
    );
}