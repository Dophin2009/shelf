let LinkType = ../File/LinkType

let Tree =
      { Type =
          { path : Text
          , defaultLinkType : Optional LinkType
          , ignorePatterns : List Text
          , replaceFiles : Optional Bool
          , replaceDirectories : Optional Bool
          }
      , default =
        { defaultLinkType = None LinkType
        , ignorePatterns = [] : List Text
        , replaceFiles = None Bool
        , replaceDirectories = None Bool
        }
      }

in  Tree
