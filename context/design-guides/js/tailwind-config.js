// Tailwind CSS Configuration
tailwind.config = {
    darkMode: "class",
    theme: {
        extend: {
            colors: {
                "primary": "#279EA7", // Brand Primary
                "secondary": "#1F3244", // Brand Secondary
                "background-light": "#f6f8f7",
                "background-dark": "#0f1821",
                "surface-dark": "#1F3244", // Brand Secondary
                "surface-light": "#ffffff",
            },
            fontFamily: {
                "display": ["Spline Sans", "sans-serif"]
            },
            borderRadius: {
                "DEFAULT": "1rem",
                "lg": "1.5rem",
                "xl": "2rem",
                "2xl": "3rem",
                "full": "9999px"
            },
        },
    },
};
