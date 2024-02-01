const path = require("path");
const CopyPlugin = require("copy-webpack-plugin");

module.exports = {
    mode: "development",
    watch: true,
    entry: "./src/sw.ts",
    module: {
        rules: [
            {
                use: "ts-loader",
                exclude: /node_modules/,
            },
        ],
    },
    output: {
        filename: "sw.js",
        path: path.resolve(__dirname, "./dist/"),
    },
    resolve: {
        extensions: [".ts", ".js"],
    },
    plugins: [
        new CopyPlugin({
            patterns: [{ from: "static"}],
        }),
    ],
};
