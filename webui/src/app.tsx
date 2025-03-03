import { Navbar } from './components/Navbar'
import { AddLinkForm } from './components/AddLinkForm'
import Footer from './components/Footer'

export function App() {  

  return (
    <div class="flex flex-col items-center justify-between h-svh bg-gray-900">
      <Navbar />
      <AddLinkForm />
      <Footer />
    </div>
  )
}
