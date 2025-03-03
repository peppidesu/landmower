import { render } from 'preact'
import './index.css'
import { App } from './app.tsx'
import Spinner from "../assets/spinner.svg";

render(<App />, document.getElementById('app')!)
