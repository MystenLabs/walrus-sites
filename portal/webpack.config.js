const path = require("path");
const HtmlMinimizerPlugin = require("html-minimizer-webpack-plugin");
const CopyPlugin = require("copy-webpack-plugin");
const CssMinimizerPlugin = require("css-minimizer-webpack-plugin");

module.exports = {
    mode: "development",
    watch: true,
    entry: {
        sw: "./src/sw.ts",
    },
    module: {
        rules: [
            {
                test: /\.html$/,
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
        clean: true,
    },
    resolve: {
        extensions: [".ts", ".js"],
    },
    plugins: [
        new CopyPlugin({
            patterns: [
                {
                    from: "static",
                    globOptions: { ignore: ["**/*.template.html"] },
                },
            ],
        }),
    ],
    optimization: {
        minimizer: [`...`, new HtmlMinimizerPlugin(), new CssMinimizerPlugin()],
    },
};
