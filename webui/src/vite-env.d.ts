/// <reference types="vite/client" />
/// <reference types="vite-plugin-svgr/client" />

interface ImportMetaEnv {
    VITE_SERVER_URL: string    
}
interface ImportMeta {
    env: ImportMetaEnv
}