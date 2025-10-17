export default {
    theme: {
        extend: {
            spacing: { 'space': '0.75rem' },
            borderRadius: { 'xl2': '1rem'},
            boxShadow: { 'soft': '0 6px 20px rgba(0, 0, 0, 0.08)'}
        }
    },
    plugins: [require('daisyui')],
    daisyui: {
        themes: ['light', 'dark', 'forest', 'night']
    }
}
