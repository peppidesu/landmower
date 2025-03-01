import { useState } from 'preact/hooks'
import preactLogo from './assets/preact.svg'
import viteLogo from '/vite.svg'
import { Navbar } from './components/Navbar'
import { GenerateForm } from './components/GenerateForm'
import Footer from './components/Footer'

export function App() {  

  return (
    <div class="flex flex-col items-center justify-between h-svh bg-gray-900">
      <Navbar />
      <GenerateForm />
      <Footer />
    </div>
  )
}
