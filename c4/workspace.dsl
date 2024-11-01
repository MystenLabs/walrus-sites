workspace "Walrus Sites" "The Walrus Sites architecture, as a C4 model." {

    !identifiers hierarchical

    model {
        u = person "User"
        walrus_sites = softwareSystem "Walrus Sites" {
            walrus = container "Walrus" {
                tags "Database"
            }
            sui = container "Sui Smart Contract"
        }

        u -> walrus_sites.walrus "Uses"
        walrus_sites.walrus -> walrus_sites.sui "Reads from and writes to"
    }

    views {
        systemContext walrus_sites "Diagram1" {
            include *
            autolayout lr
        }

        container walrus_sites "Diagram2" {
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
            }
            element "Container" {
                background #f8289c
            }
                element "Database" {
                shape cylinder
            }
        }
    }

    configuration {
        scope softwaresystem
    }

}
