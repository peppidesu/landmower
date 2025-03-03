import { defineConfig } from 'vite'
import preact from '@preact/preset-vite'
import tailwindcss from '@tailwindcss/vite'
import svgr from 'vite-plugin-svgr'

// https://vite.dev/config/
export default defineConfig(({ command, mode, isSsrBuild, isPreview }) => {
  return {
    base: (mode === 'production') ? '/static/' : '/',    
    plugins: [
      preact(), 
      tailwindcss(), 
      svgr({
        svgrOptions: {

        }        
      })
    ],
    build: {
      outDir: '../static',
      emptyOutDir: true
    },
  }
})
