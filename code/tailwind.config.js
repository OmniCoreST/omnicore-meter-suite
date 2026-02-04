/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{svelte,js,ts,jsx,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        primary: "#279EA7",
        secondary: "#1F3244",
        "background-light": "#f6f8f7",
        "background-dark": "#0f1821",
        "surface-dark": "#1F3244",
        "surface-light": "#ffffff",
      },
      fontFamily: {
        display: ["Oxanium", "sans-serif"],
        sans: ["Oxanium", "sans-serif"],
      },
      borderRadius: {
        DEFAULT: "0.25rem",
        sm: "0.125rem",
        md: "0.25rem",
        lg: "0.375rem",
        xl: "0.5rem",
        "2xl": "0.75rem",
        "3xl": "1rem",
        full: "9999px",
      },
    },
  },
  plugins: [],
};
