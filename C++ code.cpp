#include <iostream>
#include <string>
#include <vector>
#include <cstdlib>
#include <ctime>

using namespace std;

struct Building {
    string name;
    char symbol;
    int hp;
    int price;
    int residents;
    int income;
    int attraction;
    int buildTime;
    bool factory;
};

struct Construction {
    Building building;
    int days;
};

void showMap(vector<Building>& city) {
    cout << "\n====== CITY MAP ======\n";
    cout << "+---------+\n";

    int slot = 0;

    for (int y = 0; y < 4; y++) {
        cout << "| ";

        for (int x = 0; x < 4; x++) {
            if (slot < city.size())
                cout << city[slot].symbol << " ";
            else
                cout << ". ";

            slot++;
        }

        cout << "|\n";
    }

    cout << "+---------+\n";

    cout << "H-House  B-Block  A-Apartment\n";
    cout << "P-Park R-Restaurant L-Field\n";
    cout << "F-Factory\n";
}

void showBuildings(vector<Building>& city) {
    cout << "\n===== YOUR BUILDINGS =====\n";

    if (city.empty()) {
        cout << "No buildings\n";
    }

    for (int i = 0; i < city.size(); i++) {
        cout << i + 1 << ". "
             << city[i].name
             << " HP:"
             << city[i].hp
             << "\n";
    }
}

void showConstructions(vector<Construction>& constructions) {
    cout << "\n===== CONSTRUCTION =====\n";

    if (constructions.empty()) {
        cout << "No constructions\n";
    }

    for (int i = 0; i < constructions.size(); i++) {
        cout << constructions[i].building.name
             << " - days left: "
             << constructions[i].days
             << "\n";
    }
}

int main() {

    srand(time(0));

    vector<Building> city;
    vector<Construction> constructions;

    vector<Building> shop = {
        {"House",      'H', 100, 50,  5,  0,  0,  1, false},
        {"Block",      'B', 100, 120, 20, 1,  0,  3, false},
        {"Apartment",  'A', 100, 200, 40, 2,  10, 5, false},
        {"Park",       'P', 100, 80,  0,  1,  15, 2, false},
        {"Restaurant", 'R', 100, 150, 5,  5,  20, 3, false},
        {"Field",      'L', 100, 60,  0,  1,  0,  2, false},
        {"Factory",    'F', 100, 300, 10, 10, -5, 7, true}
    };

    int money = 500;
    int people = 0;
    int attraction = 0;
    int points = 0;
    int day = 1;
    int bulldozers = 0;

    int choice;

    cout << "============================\n";
    cout << "   CITY CATACLYSM 6.0\n";
    cout << "============================\n";

    while (true) {

        cout << "\n====================\n";
        cout << "Day: " << day << "\n";
        cout << "Money: " << money << "\n";
        cout << "People: " << people << "\n";
        cout << "Attraction: " << attraction << "\n";
        cout << "Bulldozers: " << bulldozers << "\n";
        cout << "Points: " << points << "\n";
        cout << "====================\n";

        showMap(city);
        showBuildings(city);
        showConstructions(constructions);

        cout << "\nMENU\n";
        cout << "1. Buy building\n";
        cout << "2. Buy bulldozer\n";
        cout << "3. Repair building\n";
        cout << "4. Next day\n";
        cout << "5. Disaster\n";
        cout << "6. Exit\n";

        cout << "Choice: ";
        cin >> choice;

        if (choice == 1) {

            cout << "\n===== SHOP =====\n";

            for (int i = 0; i < shop.size(); i++) {
                cout << i + 1 << ". "
                     << shop[i].name
                     << " Price:"
                     << shop[i].price
                     << " Build time:"
                     << shop[i].buildTime
                     << " days\n";
            }

            int buy;

            cout << "Number: ";
            cin >> buy;

            if (buy >= 1 && buy <= shop.size()) {

                if (money >= shop[buy - 1].price) {

                    money -= shop[buy - 1].price;

                    constructions.push_back(
                        {shop[buy - 1],
                         shop[buy - 1].buildTime}
                    );

                    cout << "Construction started!\n";
                }
                else {
                    cout << "Not enough money!\n";
                }
            }
        }

        else if (choice == 2) {

            cout << "A bulldozer costs 200 coins\n";

            if (money >= 200) {
                money -= 200;
                bulldozers++;

                cout << "Disaster bulldozer purchased!\n";
            }
            else {
                cout << "Not enough money!\n";
            }
        }

        else if (choice == 3) {

            if (city.empty()) {
                cout << "You have no buildings!\n";
            }
            else {

                showBuildings(city);

                int number;

                cout << "Number of the building to repair: ";
                cin >> number;

                if (number >= 1 && number <= city.size()) {

                    if (city[number - 1].hp >= 100) {
                        cout << "This building is intact!\n";
                    }
                    else if (money >= 50) {

                        city[number - 1].hp += 25;

                        if (city[number - 1].hp > 100)
                            city[number - 1].hp = 100;

                        money -= 50;

                        cout << "Building repaired!\n";
                    }
                    else {
                        cout << "Not enough money!\n";
                    }
                }
                else {
                    cout << "No such building!\n";
                }
            }
        }

        else if (choice == 4) {

            day++;

            cout << "Day "
                 << day
                 << " has begun\n";

            // Income
            for (int i = 0; i < city.size(); i++) {
                money += city[i].income;
            }

            // Constructions
            for (int i = 0; i < constructions.size(); i++) {

                constructions[i].days--;

                if (constructions[i].days <= 0) {

                    cout << "Building completed: "
                         << constructions[i].building.name
                         << "\n";

                    city.push_back(constructions[i].building);

                    people += constructions[i].building.residents;
                    attraction += constructions[i].building.attraction;

                    constructions.erase(constructions.begin() + i);

                    i--;
                }
            }
        }

        else if (choice == 5) {

            string disasters[] = {
                "Flood",
                "Earthquake",
                "Meteorite",
                "Hurricane",
                "Fire",
                "Factory malfunction"
            };

            int factories = 0;

            for (int i = 0; i < city.size(); i++) {
                if (city[i].factory)
                    factories++;
            }

            int roll;

            if (factories > 0 && rand() % 2 == 0)
                roll = 5;
            else
                roll = rand() % 5;

            int damage = rand() % 40 + 20;

            if (roll == 5)
                damage = 70;

            if (bulldozers > 0) {
                cout << "The bulldozer reduced the disaster!\n";

                damage /= 2;
                bulldozers--;
            }

            cout << "\n!!! DISASTER !!!\n";
            cout << disasters[roll] << "\n";
            cout << "Damage: -"
                 << damage
                 << " HP\n";

            int daysLost;

            if (roll == 0)
                daysLost = 1;
            else if (roll == 1)
                daysLost = 3;
            else if (roll == 2)
                daysLost = 5;
            else if (roll == 3)
                daysLost = 2;
            else
                daysLost = 4;

            day += daysLost;

            cout << "The disaster takes away "
                 << daysLost
                 << " days!\n";

            for (int i = 0; i < city.size(); i++) {

                city[i].hp -= damage;

                if (city[i].hp <= 0) {

                    cout << "Destroyed: "
                         << city[i].name
                         << "\n";

                    people -= city[i].residents;
                    attraction -= city[i].attraction;

                    city.erase(city.begin() + i);

                    i--;
                }
            }

            if (people < 0)
                people = 0;

            if (attraction < 0)
                attraction = 0;

            points += 30;
        }

        else if (choice == 6) {

            cout << "\n===== GAME OVER =====\n";

            cout << "Days: " << day << "\n";
            cout << "Points: " << points << "\n";
            cout << "People: " << people << "\n";
            cout << "Money: " << money << "\n";
            cout << "Attraction: " << attraction << "\n";

            break;
        }

        else {
            cout << "No such option!\n";
        }

        if (people <= 0 && day > 5) {

            cout << "\nTHE CITY HAS FALLEN!\n";
            cout << "No residents left.\n";

            break;
        }
    }

    return 0;
}