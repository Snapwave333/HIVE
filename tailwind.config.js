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
          base: "#0B0F14",
          panel: "#121820",
          surface: "#18222D",
          surfaceHover: "#1E2A38",
          outgoing: "#16332E",
          outgoingBorder: "#204D45",
          incoming: "#18222D",
          border: "#212E3D",
          cyan: "#42CFC4",
          lime: "#85D94B",
          dimText: "#7E8F9F",
          brightText: "#E0E8F0",
          highlight: "#2A3C4E"
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
