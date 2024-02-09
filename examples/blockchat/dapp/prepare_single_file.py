from argparse import ArgumentParser
from os import path, listdir

OUT_FILE = "index.html"


def parse_args():
    parser = ArgumentParser()
    parser.add_argument(
        "-i", "--html", type=str, required=False, default="dist/index.html"
    )
    parser.add_argument(
        "-s", "--src-dir", type=str, required=False, default="dist/assets"
    )
    parser.add_argument(
        "-o", "--out-dir", type=str, required=False, default="single-html"
    )
    return parser.parse_args()


def prepare_single_file(html, src_dir, out_dir):
    with open(html, 'r') as f:
        contents = f.readlines()
    css = next(filter(lambda x: x.endswith(".css"), listdir(src_dir)))
    js = next(filter(lambda x: x.endswith(".js"), listdir(src_dir)))
    inline_css(contents, path.join(src_dir, css))
    inline_js(contents, path.join(src_dir, js))
    print("".join(contents))


def inline_css(contents: list[str], css_file: str):
    return inline_at_position(
        contents,
        css_file,
        '<link rel="stylesheet"',
        '<style>',
        "</style>",
    )


def inline_js(contents: list[str], js_file: str):
    return inline_at_position(
        contents,
        js_file,
        '<script type="module" crossorigin',
        '<script type="module" crossorigin>',
        "</script>",
    )


def inline_at_position(
    contents: list[str], file_to_inline: str, line_match: str, before: str, after: str
):
    for idx, line in enumerate(contents):
        if line_match in line:
            contents.pop(idx)
            contents.insert(idx, before)
            with open(file_to_inline) as f:
                contents.insert(idx + 1, f.read())
            contents.insert(idx + 2, after)
            break
    return contents


def main():
    args = parse_args()
    prepare_single_file(args.html, args.src_dir, args.out_dir)


if __name__ == "__main__":
    main()
