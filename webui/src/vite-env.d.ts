/// <reference types="vite/client" />

interface ImportMetaEnv {
    VITE_SERVER_URL: string
}
interface ImportMeta {
    env: ImportMetaEnv
}