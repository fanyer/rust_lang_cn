use iron::prelude::*;
use base::framework::{ResponseData, temp_response, not_found_response};
use base::db::MyPool;
use persistent::Read;
use base::model::{Article, User, Category};
use mysql as my;
use mysql::QueryResult;
use rustc_serialize::json::ToJson;
use router::Router;
use base::util::gen_gravatar_url;
use base::constant;
use base::util;

pub fn index(req: &mut Request) -> IronResult<Response> {
    let pool = req.get::<Read<MyPool>>().unwrap().value();

    let result = pool.prep_exec("SELECT a.id, a.category, a.title, a.content, a.comments_count, a.create_time, \
                                 u.id as user_id, u.username, u.email from article \
                                 as a join user as u on a.user_id=u.id where a.status=? order by a.priority desc, a.create_time desc",
                                (constant::ARTICLE_STATUS::NORMAL,)).unwrap();

    index_data(req, &pool, result, None)
}

pub fn category(req: &mut Request) -> IronResult<Response> {
    let category_id = try!(req.extensions.get::<Router>().unwrap()
                       .find("category_id").unwrap()
                       .parse::<i8>().map_err(|_| not_found_response().unwrap_err()));

    if constant::CATEGORY::ALL.iter().find(|c|**c == category_id).is_none() {
        return not_found_response();
    }

    let pool = req.get::<Read<MyPool>>().unwrap().value();

    let result = pool.prep_exec("SELECT a.id, a.category, a.title, a.content, a.comments_count, a.create_time, \
                                     u.id as user_id, u.username, u.email from article \
                                     as a join user as u on a.user_id=u.id where a.status=? and a.category=? order by a.priority desc, a.create_time desc", (constant::ARTICLE_STATUS::NORMAL, category_id)).unwrap();

    index_data(req, &pool, result, Some(category_id))
}

fn index_data(req: &mut Request, pool: &my::Pool, result: QueryResult, raw_category_id: Option<i8>) -> IronResult<Response> {
    let articles: Vec<Article> = result.map(|x| x.unwrap()).map(|row| {
        let (id, category, title, content, comments_count, create_time, user_id, username, email) = my::from_row::<(_,_,_,_,_,_,_,_,String)>(row);
        Article {
            id: id,
            category: Category::from_value(category),
            title: title,
            content: content,
            comments_count: comments_count,
            user: User {
                id: user_id,
                avatar: gen_gravatar_url(&email),
                username: username,
                email: email,
                create_time: *constant::DEFAULT_DATETIME,
            },
            create_time: create_time,
            comments: Vec::new(),
        }
    }).collect();

    // get statistics info
    let users_count = my::from_row::<usize>(pool.prep_exec("SELECT count(id) as count from user", ()).unwrap().next().unwrap().unwrap());
    let articles_count = my::from_row::<usize>(pool.prep_exec("SELECT count(id) as count from article", ()).unwrap().next().unwrap().unwrap());
    let mut data = ResponseData::new(req);
    data.insert("articles", articles.to_json());
    data.insert("users_count", users_count.to_json());
    data.insert("articles_count", articles_count.to_json());

    if let Some(category_id) = raw_category_id {
        data.insert("categories", util::gen_categories_json(Some(category_id)));
    } else {
        data.insert("categories", util::gen_categories_json(None));
        data.insert("index", 1.to_json());
    }
    temp_response("index", &data)
}
