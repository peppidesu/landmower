import axios from "axios";

export type Jsend<T, F> = { status: "success"; data: T; } |
{ status: "fail"; data: F; } |
{ status: "error"; data: string; };export type Entry = {
    key: string;
    link: string;    
    metadata: {
        used: number;
        last_used: string; created: string;
    };
};

export type AddLinkResponse = Jsend<AddLinkSuccessData, AddLinkFailData>;
type AddLinkSuccessData = { key: string; entry: Entry; };
type AddLinkFailData = { link?: string; key?: string; };

export type GetLinksResponse = Jsend<Entry[], null>;
export type GetLinkResponse = Jsend<Entry, string>;
export type DeleteLinkResponse = Jsend<null, string>;

export type ValidateAddLinkResponse = Jsend<null, AddLinkFailData>;

export async function add_link(link: string, custom: boolean, customName: string): Promise<AddLinkResponse> {    
    let res = await axios.post(`${import.meta.env.VITE_SERVER_URL}api/links`, {
        link: link,
        key: custom ? customName : undefined
    });
    return res.data;
}

export async function get_links(): Promise<GetLinksResponse> {
    let res = await axios.get(`${import.meta.env.VITE_SERVER_URL}api/links`);
    return res.data;
}

export async function get_link(key: string): Promise<GetLinkResponse> {
    let res = await axios.get(`${import.meta.env.VITE_SERVER_URL}api/links/${key}`);
    return res.data;
}

export async function delete_link(key: string): Promise<DeleteLinkResponse> {
    let res = await axios.delete(`${import.meta.env.VITE_SERVER_URL}api/links/${key}`);
    return res.data;
}

export async function validate_add_link(link: string, custom: boolean, customName: string): Promise<ValidateAddLinkResponse> {
    let res = await axios.post(`${import.meta.env.VITE_SERVER_URL}api/validate/add_link`, {
        link: link,
        key: custom ? customName : undefined
    });
    return res.data;
}