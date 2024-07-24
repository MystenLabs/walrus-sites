// The middleware will execute for every path matched by the config.matcher
export const config = {
  matcher: '/',
};

export default async function middleware(request: Request) {
    return Response.redirect(new URL('/test', request.url));
}
