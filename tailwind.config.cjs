/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./index.html', './src/**/*.{vue,ts,tsx}'],
  theme: {
    extend: {
      colors: {
        brand: {
          50: '#fff8f2',
          100: '#ffeedf',
          200: '#ffdcbc',
          300: '#ffc38f',
          400: '#f7a965',
          500: '#ea8838',
          600: '#d87327',
          700: '#b95b21',
          800: '#95491f',
          900: '#783d1d'
        }
      },
      boxShadow: {
        soft: '0 12px 32px rgba(180, 112, 52, 0.12)',
        panel: '0 20px 40px rgba(145, 84, 33, 0.15)'
      },
      fontFamily: {
        sans: ["'Space Grotesk'", "'Noto Sans SC'", "'Segoe UI'", 'sans-serif'],
        mono: ["'JetBrains Mono'", "'Consolas'", "'Courier New'", 'monospace']
      }
    }
  },
  plugins: []
};
