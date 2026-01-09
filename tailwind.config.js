/** @type {import('tailwindcss').Config} */
export default {
  darkMode: 'class',
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // macOS Dark Mode Palette
        sys: {
            bg: '#1E1E1E',       // Standard Window BG
            sidebar: '#2C2C2C',  // Sidebars
            card: '#282828',
            hover: '#3A3A3A',
            border: '#454545',   // Subtle separators
            
            // System Colors
            blue: '#0A84FF',     // macOS System Blue
            red: '#FF453A',
            green: '#30D158',
            gray: '#98989D',
            white: '#FFFFFF',
            label: 'rgba(255, 255, 255, 0.85)',
            secondary: 'rgba(255, 255, 255, 0.55)',
            tertiary: 'rgba(255, 255, 255, 0.25)',
        }
      },
      fontFamily: {
        sans: ['-apple-system', 'BlinkMacSystemFont', 'San Francisco', 'Helvetica Neue', 'sans-serif'],
      },
      boxShadow: {
         'sm': '0 1px 2px 0 rgba(0, 0, 0, 0.2)',
         'card': '0 2px 8px rgba(0, 0, 0, 0.3)',
      },
      backdropBlur: {
          'xs': '2px',
      }
    },
  },
  plugins: [],
}
