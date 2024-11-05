workspace "Walrus Sites" "The Walrus Sites architecture, as a C4 model." {

    !identifiers hierarchical

    model {
        u = person "User"
        ws = softwareSystem "Walrus Sites" {
            smart_contract = container "Sui Smart Contract" "Keeps track of each resource metadata." "Move" {
                site_object = component "Site Object" {
                    tags "move_object"
                }
                resource_object = component "Resource Object" {
                    tags "move_object"
                }
            }
            walrus = container "Walrus" "Decentralized storage network that stores and delivers raw data and media files. In this case, it stores the blobs of the site resources."
            site_builder = container "site-builder" {
                description "CLI tool that builds the site objects and uploads them to Sui and Walrus respectively."
            }

            portals = container "Portals" "Enables users to access the site resources." {
                service_worker_portal = component "service-worker-portal" "service-worker" {
                    tags "browser"
                    description "Gets installed on the user's device. Intervene between fetch events, serving the site resources."
                }
                server_side_portal = component "server-side-portal" "next.js"{
                    tags "server"
                    description "Loads first the site resources to the server and then provides them to the user."
                }
                common_lib = component "common-lib" "node.js" {
                    tags "library"
                    description "Includes functions that are common to both service-worker-portal and server-side-portal such as fetchings sui objects, contacting a walrus aggregator to fetch the files (blobs) etc."
                }
            }
        }

        u -> ws.site_builder "Uploads (writes) sites to"
        ws.portals -> u "Serves (reads) site resources to"
        ws.smart_contract -> ws.portals "Collects resource metadata from"
        ws.walrus -> ws.portals "Sends blob(s) of resources to"
        ws.site_builder -> ws.smart_contract "Uploads the resource metadata to the blockchain"
        ws.site_builder -> ws.walrus "Adds blob(s) to"

        ws.portals.common_lib -> ws.portals.server_side_portal "Provides interface for walrus & sui to"
        ws.portals.common_lib -> ws.portals.service_worker_portal "Provides interface for walrus & sui to"
        ws.portals.server_side_portal -> u "Serves (reads) site resources to"
        ws.portals.service_worker_portal -> u "Serves (reads) site resources to"
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

        component ws.smart_contract "SmartContractView" {
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
            element "move_object" {
                background #f8289c
                shape circle
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
