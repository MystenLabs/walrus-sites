workspace "Walrus Sites" "The Walrus Sites architecture, as a C4 model." {

    !identifiers hierarchical

    model {
        u = person "User"
        ws = softwareSystem "Walrus Sites" {
            smart_contract = container "Sui Smart Contract" "Move"
            walrus = container "Walrus"
            site_builder = container "site-builder"
            portals = container "Portals" {
                service_worker_portal = component "service-worker-portal" {
                    tags "browser"
                }
                server_side_portal = component "server-side-portal" {
                    tags "server"
                }
                common_lib = component "common-lib" {
                    tags "library"
                }
            }
        }

        u -> ws.site_builder "Uploads (writes) sites to"
        ws.portals -> u "Serves (reads) site resources to"
        ws.site_builder -> ws.smart_contract "Uploads the resource metadata to the blockchain"
        ws.site_builder -> ws.walrus "Adds blob(s) to"

    }

    views {
        systemContext ws "SystemView" {
            include *
            autolayout lr
        }

        container ws "WalrusSitesView" {
            include *
            autolayout lr
        }

        component ws.portals "PortalView" {
            include *
            autolayout lr
        }

        styles {
            element "Element" {
                color #ffffff
            }
            element "Person" {
                background #ba1e75
                shape person
            }
            element "Software System" {
                background #d92389
                shape component
            }
            element "browser" {
                background #f8289c
                shape webbrowser
            }
            element "server" {
                background #f8289c
                shape cylinder
            }
            element "Component" {
                background #f8289c
                shape component
            }
            element "Container" {
                background #f8289c
            }
                element "Database" {
                shape cylinder
            }
            element "library" {
                shape folder
            }
        }
    }

    configuration {
        scope softwaresystem
    }

}
