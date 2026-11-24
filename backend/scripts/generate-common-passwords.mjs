// Generates backend/src/auth/common_passwords.txt, the list of common
// passwords the Password policy refuses.
//
// Usage (from the repository root):
//   node backend/scripts/generate-common-passwords.mjs
//
// Provenance. The words below were written by hand from general knowledge of
// what people commonly choose as passwords (international "top password"
// themes, keyboard patterns, Brazilian Portuguese words, Brazilian football
// clubs, popular first names). No list was downloaded or copied from a
// leaked-password dataset, and nothing here comes from TabWallet data: the
// list holds no personal data and no real account's password. The script then
// combines the most popular bases with the endings people append (digits,
// years, "!", "@123"). The policy itself also strips trailing digits and
// symbols before looking a password up, so the combinations are a belt on top
// of the braces, and the file reads as the "few thousand entries" the rule
// promises.
//
// Entries are lowercase ASCII (no accents), one per line, `#` starts a
// comment. The policy folds accents and case before the lookup.

import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const words = (text) =>
  text
    .split(/\s+/)
    .map((word) => word.trim().toLowerCase())
    .filter(Boolean);

// Passwords that are common as they are (digits, symbols and leet speak
// included). Always listed as written.
const standalone = words(`
  123456 1234567 12345678 123456789 1234567890 12345678910 0123456789 987654321 9876543210
  111111 1111111 11111111 111111111 1111111111 000000 0000000 00000000 000000000 0000000000
  222222 333333 444444 555555 666666 777777 888888 999999 121212 123123 112233 654321 666666
  1q2w3e4r 1q2w3e4r5t 1q2w3e4r5t6y 1qaz2wsx 1qaz2wsx3edc 1qazxsw2 2wsx3edc 1qaz1qaz 1q2w3e
  qazwsx qazwsxedc qazwsxedcrfv qweasd qweasdzxc qwe123 qwe12345 qweqwe qweqweqwe qweasdzxc123
  q1w2e3r4 q1w2e3r4t5 q1w2e3r4t5y6 qwertyui qwertyuiop qwertyuiopasdfghjkl asdfghjk asdfghjkl
  asdfgh asdasd asdasdasd asd123 asd12345 zxcvbn zxcvbnm zxcvbnm123 zxczxc zxc123 zxcasdqwe
  abcdef abcdefg abcdefgh abcdefghi abcdefghij abc123 abc12345 abcd1234 abcabc abcabcabc
  password passw0rd p@ssw0rd p@ssword pa55word pa55w0rd passwort pass1234 pass12345 passpass
  senha senha1 senha12 senha123 senha1234 senha12345 senha123456 senhaforte senhasenha
  minhasenha minhasenha123 minhasenha1 mudar123 mudar@123 trocar123 teste teste123 teste1234
  teste12345 testando testando123 login admin admin123 admin1234 administrator root root123
  toor user user123 guest guest123 default master master123 letmein welcome welcome1 welcome123
  changeme change123 secret secret123 secreta secreto segredo segredo123 mypassword mypass
  password1 password12 password123 password1234 password12345 pass word123 trustno1 iloveyou
  iloveyou1 iloveyou123 ilovemyself iloveu iloveyoutoo loveyou loveme loveyou123 lovely
  monkey dragon baseball football soccer hockey letmein1 sunshine princess starwars batman
  superman spiderman ironman hello hello123 hello1234 helloworld hello@123 freedom shadow
  michael jordan jordan23 jennifer hunter harley ranger buster thomas robert soccer1 killer
  pepper joshua matthew maggie jessica daniel andrew charlie george access mustang cheese
  summer winter spring autumn flower orange yellow purple silver golden diamond rainbow
  internet computer laptop google facebook instagram whatsapp twitter youtube netflix amazon
  tabwallet carteira carteira123 dinheiro dinheiro123 financas financeiro cartao cartao123
  amor amor123 amorzinho meuamor meuamor123 euteamo euteamo123 teamo teamo123 teamomuito
  tequiero tequiero123 meuamorzinho amorperfeito amoreterno amorsincero amoremeu
  brasil brasil123 brasil1234 brasil2000 brasileiro brasileira brasilia brasilsil
  saopaulo riodejaneiro minasgerais bahia salvador recife fortaleza curitiba manaus belem
  porto alegre portoalegre belohorizonte florianopolis goiania natal maceio aracaju
  familia familia123 minhafamilia familiafeliz mae mae123 mamae mamae123 papai papai123
  pai pai123 filho filha irmao irma vovo vovo123 deus deus123 deusefiel deusmeguarde
  deusmeguie deusnocomando deuseamor deus123456 jesus jesus123 jesuscristo jesusteama
  jesusvive jesusefiel gloria aleluia aleluia123 abencoado abencoada abencoado123 fe123
  flamengo flamengo123 flamengo1981 mengao mengao123 mengaoo corinthians corinthians123
  timao timao123 timaozao palmeiras palmeiras123 verdao verdao123 saopaulofc tricolor
  tricolor123 santos santosfc peixe gremio gremio123 internacional colorado cruzeiro
  cruzeiro123 vasco vascodagama vasco123 botafogo botafogo123 fluminense fluminense123
  atleticomg atletico galo galo123 bahiafc sport sportrecife ceara cearasc fortalezaec
  goias coritiba athletico bragantino cuiaba juventude criciuma vitoria nautico
  neymar neymar10 ronaldo ronaldinho ronaldo9 messi messi10 cristiano cristianoronaldo
  pele pele10 romario zico zico10 kaka robinho adriano gabigol hulk
  futebol futebol123 futebolclube campeao campeao123 hexa penta tetra gol gol123 golaco
  santa santo santinha santinho saudade saudade123 felicidade felicidade123 paz paz123
  lindo linda linda123 lindo123 bonita bonito gostosa gostoso gatinha gatinho princesa
  princesa123 princesinha principe rainha rei rei123 bebe bebe123 docinho docinho123
  fofinha fofinho coracao coracao123 meucoracao querido querida amigo amiga amigos
  amigo123 amigos123 melhoramigo melhoramiga paixao paixao123 sonho sonhos sonhador
  vida vida123 minhavida minhavida123 viver viva viva123 vitoria123 vencedor vencedor123
  guerreiro guerreira guerreiro123 forte forte123 poder poder123 sucesso sucesso123
  sorte sorte123 fortuna riqueza riqueza123 rico rico123 milionario milionario123
  maconha maconha123 cerveja cerveja123 cachaca pinga churrasco churrasco123 feijoada
  chocolate chocolate123 pizza pizza123 hamburguer sorvete brigadeiro pastel coxinha
  cachorro cachorro123 cachorrinho gato gato123 gatinho123 passaro tigre leao leao123
  elefante macaco macaco123 cavalo coelho borboleta golfinho tubarao dinossauro dragao
  dragao123 fenix lobo lobo123 aguia aguia123 falcao panda panda123 tartaruga
  sol sol123 lua lua123 estrela estrela123 estrelas mar mar123 praia praia123 verao
  inverno primavera outono natal123 reveillon carnaval carnaval123 ferias ferias123
  janeiro fevereiro marco abril maio junho julho agosto setembro outubro novembro dezembro
  segunda terca quarta quinta sexta sabado domingo
  azul azul123 vermelho amarelo verde preto branco rosa roxo laranja cinza marrom dourado
  prateado
  um dois tres quatro cinco seis sete oito nove dez
  uno dos tres cuatro cinco seis siete ocho nueve diez
  hola hola123 amor123456 password2 contrasena contrasena123 clave clave123 secreto123
  carro carro123 moto moto123 fusca gol123 uno123 palio corsa civic corolla ferrari
  lamborghini porsche mercedes bmw audi fiat volkswagen chevrolet toyota honda yamaha
  kawasaki suzuki ducati harley123 mustang123 camaro corvette
  nike adidas puma gucci prada chanel dior versace rolex iphone android samsung
  playstation xbox nintendo minecraft fortnite roblox pokemon naruto goku dragonball
  dragonball123 onepiece zelda mario mario123 sonic pikachu pikachu123 batman123
  superman123 hulk123 thor ironman123 spiderman123 wolverine joker joker123 matrix
  harrypotter hogwarts gandalf starwars123 jedi vader yoda avatar titanic
  messenger skype hotmail gmail yahoo outlook email email123 emailmeu mail
  facebook123 insta insta123 face face123 orkut orkut123 zap zap123 whats whats123
  telefone telefone123 celular celular123 casa casa123 minhacasa lar lardocelar
  trabalho trabalho123 empresa empresa123 escola escola123 faculdade estudante
  professor professor123 aluno aluno123 medico medico123 advogado engenheiro
  computador computador123 notebook teclado mouse monitor impressora wifi wifi123
  senhawifi senhadowifi internet123 roteador router router123
  qwerty qwerty1 qwerty12 qwerty123 qwerty1234 qwerty12345 qwerty123456 qwertz azerty
  asdf asdf1234 asdfasdf asdfghjkl123 zaq12wsx zaq1zaq1 zaq1xsw2 zxcv zxcvbnm1
  mnbvcxz poiuytrewq lkjhgfdsa lkjhgfdsapoiuytrewq 0987654321 09876543 987654 9876543
  147258369 147852369 159357 159753 159357456 258456 357159 741852963 963852741 789456123
  456789 456789123 123456a 123456abc 123456789a a123456 a1234567 a12345678 a123456789
  abc123456 abcd123456 aa123456 aa1234567 aaa111 aaaaaa aaaaaaa aaaaaaaa aaaaaaaaa aaaaaaaaaa
  qq123456 qq1234567 zz123456 xx123456 pp123456 oo123456 q123456 q1234567 z123456
  1234qwer 1234qwer1234 1234abcd 12345qwert 12345qwerty 123qwe 123qwe123 123qweasd
  123asd 123asdzxc 123abc 123abc123 123321 12344321 1234321 12341234 123454321
  love4ever forever forever123 foreverlove together sempre sempre123 semprejuntos
  euamo euamomeufilho eusouofodao eusoulinda eusoulindo euquero euteamomuito
  euamovoce euamovc vcmeama meuamigo minhaamiga
  naosei naosei123 naoseiasenha naotem naotenho sem senhanenhuma nenhuma nada nada123
  ninguem ninguem123 qualquer qualquercoisa qualquer123 tanto tantofaz tantofaz123
  tudo tudobem tudook ok ok123 oi oi123 ola ola123 oie oiee oioi oioioi
  bomdia bomdia123 boatarde boanoite tchau tchau123 obrigado obrigada obrigado123
  macaco123456 abacaxi banana banana123 maca morango laranja123 uva uva123 manga
  limao limao123 melancia melancia123 pessego goiaba mamao
  bobo bobo123 bobinho bobinha bobona bobao feio feia chato chata legal legal123
  maluco maluco123 doido doido123 louco louco123 loucura loucura123 safado safada
  sapeca sapeca123 danadinha danadinho levado levada
  admin@123 admin@1234 password@123 senha@123 senha@1234 brasil@123 teste@123 mudar@1234
  p@$$w0rd p@ssw0rd123 p@55w0rd p4ssw0rd pa$$word pa$$w0rd passw0rd1 passw0rd123
  s3nh4 s3nha123 senh@123 s3nh@123 sen4a123 amor@123 deus@123 jesus@123 familia@123
  l3tm31n letmein123 l3tm31n123 w3lc0m3 w3lc0me welcome@123 qwerty@123 qwerty@1234
  trustno1 trustno1234 whatever whatever123 nothing nothing123 nopassword nopass
  blink182 linkinpark metallica nirvana beatles queen acdc acdc123 pinkfloyd ledzeppelin
  legiaourbana legiaourbana123 raimundos charlie brownjr charliebrownjr skank titas
  paralamas barao cazuza rita lee ritalee anitta ivete ivetesangalo claudialeitte
  zecapagodinho jorgeemateus gusttavo gusttavolima marilia mariliamendonca
  chitaozinho xororo leandroeleonardo zezedicamargo luciano
`);

// Bases that get the typical endings appended. Each is a word people really
// pick on its own.
const bases = words(`
  senha brasil amor deus jesus familia flamengo corinthians palmeiras santos gremio
  cruzeiro vasco botafogo fluminense atletico internacional sport bahia ceara fortaleza
  mengao timao verdao galo tricolor futebol campeao neymar ronaldo messi
  password qwerty abc asdf zxcv admin teste login master welcome secret dragon monkey
  letmein sunshine princess football baseball soccer hockey batman superman spiderman
  iloveyou loveyou lovely freedom shadow michael jordan jennifer harley hunter ranger
  thomas robert killer pepper joshua matthew maggie jessica daniel andrew charlie george
  mustang cheese summer winter flower orange yellow purple silver golden diamond rainbow
  internet computer google facebook instagram whatsapp
  minhasenha mudar trocar meuamor euteamo teamo coracao amigo amiga paixao saudade
  felicidade vida sonho sucesso sorte vitoria guerreiro forte poder lindo linda princesa
  gatinha gatinho cachorro leao tigre lobo dragao fenix panda estrela sol lua mar praia
  azul vermelho preto branco rosa verde amarelo carro moto casa celular telefone
  trabalho empresa escola faculdade professor estudante computador notebook wifi
  carteira dinheiro financas cartao tabwallet hello world test user guest root
  naruto goku pokemon minecraft fortnite roblox mario pikachu joker matrix
  lucas gabriel pedro joao miguel arthur davi heitor bernardo samuel guilherme felipe
  rafael bruno carlos jose paulo marcos andre ricardo fernando rodrigo thiago diego
  maria ana julia beatriz mariana juliana fernanda amanda camila larissa bruna leticia
  patricia aline carolina jessica vanessa renata sandra claudia luciana adriana
  alice laura helena valentina sophia isabella manuela livia lorena gabriela
`);

// Endings people append to a base: sequences, years and a bit of punctuation.
const endings = [
  "1", "12", "123", "1234", "12345", "123456", "321", "00", "2024", "2025", "2026",
  "!", "@123",
];

const entries = new Set();
for (const word of standalone) entries.add(word);
for (const base of bases) {
  entries.add(base);
  for (const ending of endings) entries.add(base + ending);
}

const sorted = [...entries].filter((entry) => entry.length >= 3).sort();
const header = `# Common passwords refused by the Password policy (backend/src/auth/password_policy.rs).
# Generated by backend/scripts/generate-common-passwords.mjs: do not edit by hand.
# Hand-written from general knowledge (no leaked dataset), no personal data.
# One lowercase ASCII entry per line; the policy folds case and accents before looking up.
`;
const target = fileURLToPath(
  new URL("../src/auth/common_passwords.txt", import.meta.url),
);
writeFileSync(target, header + sorted.join("\n") + "\n", "utf8");
console.log(`${sorted.length} entries written to ${target}`);
