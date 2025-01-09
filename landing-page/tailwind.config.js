// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.{js,jsx,ts,tsx}"],
  theme: {
    extend: {
      colors: {
        primary_dark: "#0C0F1D",
        primary_pink: "#C684F6",
        primary_teal: "#97F0E5",
        gradientStart: "#97F0E5",
        gradientEnd: "#578A84",
      },
      fontFamily: {
        ppMondwest: ["PPMondwest-Regular", "sans-serif"],
        ppMondwestBold: ["PPMondwest-Bold", "sans-serif"],
        ppNeueBit: ["PPNeueBit-Regular", "sans-serif"],
        ppNeueBitBold: ["PPNeueBit-Bold", "sans-serif"],
        ppNeueMontreal: ["PPNeueMontreal-Regular", "sans-serif"],
        ppNeueMontrealBold: ["PPNeueMontreal-Bold", "sans-serif"],
        ppNeueMontrealThin: ["PPNeueMontreal-Thin", "sans-serif"],
      },
      screens: {
        custom_xl: "1200px",
        custom_lg: "1100px",
        custom_md: "660px",
      },
    },
  },
  plugins: [],
};
