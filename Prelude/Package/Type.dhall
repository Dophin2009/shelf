let Hook = ../Hook/Type.dhall

let File = ../File/Type

let LinkType = ../File/LinkType

let Template = ../Template/Type

let Tree = ../Tree/Type

let Tree/default = ../Tree/default

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
          , trees : List Tree.Type
          }
      , default =
        { dependencies = [] : List Text
        , defaultLinkType = LinkType.Link
        , ignorePatterns = [] : List Text
        , files = [] : List File.Type
        , templateFiles = [] : List Template.Type
        , beforeLink = [] : List Hook
        , afterLink = [] : List Hook
        , trees = [ Tree/default ]
        }
      }

in  Package
