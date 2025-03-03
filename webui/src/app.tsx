import Navbar from './components/Navbar'
import AddLinkForm from './components/AddLinkForm'
import Footer from './components/Footer'
import { ErrorBoundary, LocationProvider, Route, Router } from 'preact-iso'
import ManageLinks from './components/ManageLinks'

export function App() {  
  console.log(import.meta.env.BASE_URL)
  return (
    <div class="flex flex-col items-center justify-between h-svh bg-gray-900">
      <Navbar />
      <LocationProvider>
        <ErrorBoundary>
          <Router>
            <Route path="/" component={AddLinkForm} />
            <Route path="/manage" component={ManageLinks} />
          </Router>
        </ErrorBoundary>
      </LocationProvider>
      <Footer />
    </div>
  )
}
