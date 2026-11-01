/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        hive: {
          base: "#000000",
          panel: "#060606",
          surface: "#0D0D0D",
          surfaceHover: "#151515",
          outgoing: "#0B1606",
          outgoingBorder: "#1D3612",
          incoming: "#0A0A0A",
          border: "#1A1A1A",
          accent: "#85D94B",
          cyan: "#85D94B",
          lime: "#85D94B",
          dimText: "#7A7A7A",
          brightText: "#F2F2F2",
          highlight: "#171717"
        }
      },
      fontFamily: {
        segoe: ["Segoe UI Variable", "Segoe UI", "sans-serif"],
        atkinson: ["Atkinson Hyperlegible", "sans-serif"],
        dyslexic: ["OpenDyslexic", "sans-serif"]
      }
    },
  },
  plugins: [],
}
