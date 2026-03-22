module ExcessArgs where

createUser :: String -> String -> Int -> String -> String -> String -> Bool -> String -> String
createUser fname lname age email phone addr active role =
  fname ++ " " ++ lname ++ " " ++ show age ++ " " ++ email ++ " " ++ phone ++ " " ++ addr ++ " " ++ show active ++ " " ++ role

simpleFunc :: Int -> Int
simpleFunc x = x + 1
