const path = require("path");
const CopyPlugin = require("copy-webpack-plugin");

module.exports = {
    mode: "development",
    watch: true,
    entry: {
        sw: "./src/sw.ts",},
    module: {
        rules: [
            {
                test: /\.txt$/,
                type: "asset/source",
            },
            {
                test: /\.ts$/,
                use: "ts-loader",
                exclude: /node_modules/,
            },
        ],
    },
    output: {
        filename: "[name].js",
        path: path.resolve(__dirname, "./dist/"),
    },
    resolve: {
        extensions: [".ts", ".js"],
    },
    plugins: [
        new CopyPlugin({
            patterns: [{ from: "static" }],
        }),
    ],
};
