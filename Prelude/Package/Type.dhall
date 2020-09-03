let Hook = ../Hook/Type.dhall

let File = ../File/Type

let LinkType = ../File/LinkType

let Template = ../Template/Type

let Package =
      { Type =
          { name : Text
          , dependencies : List Text
          , defaultLinkType : LinkType
          , ignorePatterns : List Text
          , files : List File.Type
          , templateFiles : List Template.Type
          , beforeLink : List Hook
          , afterLink : List Hook
          , treePath : Text
          }
      , default =
        { dependencies = [] : List Text
        , defaultLinkType = LinkType.Link
        , ignorePatterns = [] : List Text
        , files = [] : List File.Type
        , templateFiles = [] : List Template.Type
        , beforeLink = [] : List Hook
        , afterLink = [] : List Hook
        , treePath = "tree"
        }
      }

in  Package
