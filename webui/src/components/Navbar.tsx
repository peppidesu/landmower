
export default function Navbar() {
    return <div class="bg-gray-800 text-white p-3 flex justify-between items-center w-full">
        <h1 class="text-2xl">landmower</h1>
        <nav class="flex items-center gap-2">
            <a href="/">
                <i class="ti ti-link-plus navbar-button"></i>
            </a>
            <a href="/manage">
                <i class="ti ti-list-search navbar-button"></i>
            </a>
            <a href="https://github.com/peppidesu/landmower.git">
                <i class="ti ti-brand-github navbar-button"></i>
            </a>
        </nav>
    </div>
}